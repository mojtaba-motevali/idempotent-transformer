import {
  IdempotentCheckSumGenerator,
  IdempotentLogger,
  IdempotentSerializer,
  IdempotentStateStore,
} from '../base';
import {
  IdempotencyResult,
  IdempotentTransformerInput,
  IdempotentTransformerOptions,
  MakeIdempotentResult,
} from './interfaces';

export class IdempotentTransformer {
  private storage: IdempotentStateStore;
  private serializer: IdempotentSerializer;
  private logger?: IdempotentLogger;
  private checksumGenerator: IdempotentCheckSumGenerator;

  constructor(input: IdempotentTransformerInput) {
    const { storage, serializer, log, checksumGenerator } = input;
    this.storage = storage;
    this.serializer = serializer;
    this.logger = log;
    this.checksumGenerator = checksumGenerator;
  }

  /**
   * Makes a set of functions idempotent.
   * @param workflowId - The workflow id.
   * @param functions - The functions to make idempotent. For class methods, use the class method inside an arrow function.
   * @param options - The options for the transformer.
   * @returns The result of the transformation function.
   * @example
   * ```ts
   * const result = transformer.makeIdempotent('my-workflow-id', {
   *  exampleMethod: (...args: Parameters<typeof ExampleClass.exampleMethod>) => this.exampleMethod(...args)
   * });
   * ``
   */
  async makeIdempotent<T extends Record<string, (...args: any[]) => Promise<any> | any>>(
    workflowId: string,
    functions: T,
    { prefetchCheckpoints = false, retentionTime }: IdempotentTransformerOptions = {}
  ): Promise<MakeIdempotentResult<T>> {
    // 1 day
    const expiredAt = new Date(Date.now() + (retentionTime || 1000 * 60 * 60 * 24));
    const callbacks = functions as Record<string, (...args: any[]) => Promise<any>>;

    const result = {
      complete: async () => {
        await this.storage.complete(workflowId, expiredAt.getTime());
      },
    } as MakeIdempotentResult<T>;

    let results: IdempotencyResult[] = [];
    if (prefetchCheckpoints) {
      results = await this.storage.findAll(workflowId);
    }
    const functionNames = Object.keys(callbacks);

    for (const key of functionNames as Array<keyof T>) {
      let taskResult: IdempotencyResult | undefined = undefined;
      if (prefetchCheckpoints) {
        const taskId = await this.createCheckSum({
          workflowId,
          contextName: key as string,
        });
        taskResult = results.find((result) => result.taskId === taskId);
      }
      result[key] = this.createTransformer(
        workflowId,
        key as string,
        callbacks[key as string],
        taskResult
      ) as MakeIdempotentResult<T>[typeof key];
    }

    return result;
  }

  async createCheckSum<T>(value: T): Promise<string | number> {
    const serializedValue = await this.serializer.serialize(value);
    return this.checksumGenerator.generate(serializedValue);
  }

  /**
   * Creates a new idempotent transformer
   * @param transformFn The transformation function
   * @returns An IdempotentTransformer instance
   */
  private createTransformer =
    <F extends (...args: any[]) => Promise<any>>(
      workflowId: string,
      contextName: string,
      callbackFn: F,
      previousResult: Pick<IdempotencyResult, 'value' | 'taskId'> | null = null
    ) =>
    /**
     * The transformed task
     * @param input - The input to the task
     * @param options - The options for the transformer
     * @returns The result of the transformation function
     */
    async (...args: [...Parameters<F>]): Promise<ReturnType<F>> => {
      const input = args as unknown as Parameters<F>;
      let taskUniqueId = previousResult?.taskId || null;
      let cachedResult: IdempotencyResult | null = previousResult
        ? {
            taskId: previousResult.taskId,
            workflowId,
            value: previousResult.value,
          }
        : null;
      this.logger?.debug(
        `Creating transformer for workflow ${workflowId} and context ${contextName}`
      );
      /**
       * The task unique id is a hash of the workflow id and the idempotency key.
       */
      if (!taskUniqueId) {
        taskUniqueId = (
          await this.createCheckSum({
            workflowId,
            contextName,
          })
        ).toString();
      }

      this.logger?.debug(`Task unique id: ${taskUniqueId}`);
      if (!cachedResult) {
        cachedResult = await this.storage.find({ taskId: taskUniqueId, workflowId });
      }

      this.logger?.debug(`Cached result: ${cachedResult}`);
      if (cachedResult) {
        this.logger?.debug(`Found cached result for task ${taskUniqueId}`);
        const deserializedResult = await this.serializer?.deserialize<ReturnType<F>>(
          cachedResult.value
        );

        this.logger?.debug(`Deserialized result: ${deserializedResult}`);
        return deserializedResult;
      }

      /**
       * ================================================
       * Execution
       * ================================================
       */

      let result: ReturnType<F>;
      const tempResult = callbackFn(...input);
      if (tempResult instanceof Promise) {
        result = await tempResult;
      } else {
        result = tempResult;
      }

      /**
       * ================================================
       * Saving result
       * ================================================
       */

      this.logger?.debug(`Execution was successful, saving result to storage`);
      const idempotentResult: ReturnType<F> = result;

      const serializedIdempotentResult = await this.serializer.serialize(idempotentResult);

      this.logger?.debug(`Serialized result: ${serializedIdempotentResult}`);

      await this.storage.save(
        { taskId: taskUniqueId, workflowId, value: serializedIdempotentResult },
        {
          context: {
            taskName: contextName,
          },
          workflowId,
        }
      );
      this.logger?.debug(`Saved result for task ${taskUniqueId}`);
      return result;
    };
}
