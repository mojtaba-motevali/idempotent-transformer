import { IdempotentCompressor } from '../base/compressor';
import { IdempotentCrypto } from '../base/crypto';
import { IdempotentLogger } from '../base/logger';
import { IdempotentSerializer } from '../base/serializer';
import { IdempotentStateStore } from '../base/state-store';
import { TSerialized } from '../base/types/serialized.type';
import { IdempotencyConflictException } from './exceptions/conflict.exception';
import { IdempotencyResult } from './interfaces/idempotency-result.interface';
import { IOptions } from './interfaces/idempotent-options.interface';
import { IdempotentTransformerInput } from './interfaces/idempotent-transformer.interface';
import {
  IdempotentTransformerOptions,
  MakeIdempotentResult,
} from './interfaces/make-idempotent.interface';

export class IdempotentTransformer {
  private static instance: IdempotentTransformer;
  private storage: IdempotentStateStore;
  private serializer: IdempotentSerializer;
  private compressor?: IdempotentCompressor;
  private logger?: IdempotentLogger;
  private crypto: IdempotentCrypto;

  private constructor(input: IdempotentTransformerInput) {
    const { storage, serializer, compressor, log, crypto } = input;
    this.storage = storage;
    this.serializer = serializer;
    this.compressor = compressor;
    this.logger = log;
    this.crypto = crypto;
  }

  /**
   * Checks if the last argument in the array matches the shape of IOptions.
   * Returns true if the last argument is an object and has at least one property from IOptions.
   */
  private isLastArgOptions(args: any[]): boolean {
    if (!args.length) return false;
    const lastArg = args[args.length - 1];
    if (typeof lastArg !== 'object' || lastArg === null) return false;
    const optionKeys: (keyof IOptions)[] = ['shouldCompress'];
    return optionKeys.some((key) => key in lastArg);
  }

  static configure(input: IdempotentTransformerInput) {
    IdempotentTransformer.instance = new IdempotentTransformer(input);
  }

  static getInstance(input?: IdempotentTransformerInput) {
    if (!IdempotentTransformer.instance && input) {
      const { storage, serializer, compressor, log, crypto } = input;
      IdempotentTransformer.instance = new IdempotentTransformer({
        storage,
        serializer,
        compressor,
        log,
        crypto,
      });
    }
    if (!IdempotentTransformer.instance) {
      throw new Error(
        'IdempotentTransformer not configured, make sure you have used IdempotentFactory.build() to configure the transformer'
      );
    }
    return IdempotentTransformer.instance;
  }

  makeIdempotent<T extends Record<string, (...args: any[]) => Promise<any>>>(
    workflowId: string,
    functions: T,
    options: IdempotentTransformerOptions = {
      ttl: 1000 * 60 * 60,
    }
  ): MakeIdempotentResult<T> {
    const callbacks = functions as Record<string, (...args: any[]) => Promise<any>>;

    const result = {} as MakeIdempotentResult<T>;

    for (const key of Object.keys(callbacks) as Array<keyof T>) {
      // The type of callbacks[key] is T[key], which is (...args: any[]) => Promise<any>
      // We need to ensure the result[key] is of type (...args: [...Parameters<T[key]>, (IOptions | undefined)?]) => Promise<ReturnType<T[key]>>
      result[key] = this.createTransformer(workflowId, key as string, callbacks[key as string], {
        ttl: options.ttl,
      }) as MakeIdempotentResult<T>[typeof key];
    }

    return result;
  }

  private async compressIfEnabled(
    shouldCompress: boolean,
    value: TSerialized
  ): Promise<TSerialized> {
    if (this.compressor && shouldCompress) {
      return this.compressor.compress(value);
    }
    return value;
  }

  private async decompressIfCompressed(value: TSerialized): Promise<TSerialized> {
    if (this.compressor && this.compressor.isCompressed(value)) {
      return this.compressor.decompress(value);
    }
    return value;
  }

  async createHash<T>(value: T): Promise<string> {
    const serializedValue = await this.serializer.serialize(value);
    return this.crypto.createHash(serializedValue);
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
      { ttl }: IdempotentTransformerOptions
    ) =>
    /**
     * The transformeed task
     * @param input - The input to the task
     * @param options - The options for the transformer
     * @returns The result of the transformation function
     */
    async (...args: [...Parameters<F>, IOptions?]): Promise<ReturnType<F>> => {
      const isLastArgOptions = this.isLastArgOptions(args);
      let input: Parameters<F>;
      let options: IOptions | undefined;
      if (isLastArgOptions) {
        options = args.pop() as IOptions;
        input = args as unknown as Parameters<F>;
      } else {
        input = args as unknown as Parameters<F>;
      }

      this.logger?.debug(
        `Creating transformer for workflow ${workflowId} and context ${contextName}`
      );
      /**
       * The task unique id is a hash of the workflow id and the idempotency key.
       */
      const taskUniqueId = await this.createHash({
        workflowId,
        contextName,
      });
      /**
       * The input hash is a hash of the input.
       */
      const inputHash = await this.createHash(input);

      this.logger?.debug(`Task unique id: ${taskUniqueId}`);
      const cachedResult = await this.storage.find(taskUniqueId);
      this.logger?.debug(`Cached result: ${cachedResult}`);
      if (cachedResult) {
        this.logger?.debug(`Found cached result for task ${taskUniqueId}`);
        const decompressedResult = await this.decompressIfCompressed(cachedResult);
        const deserializedResult =
          await this.serializer?.deserialize<IdempotencyResult<ReturnType<F>>>(decompressedResult);

        if (deserializedResult.executionInputHash !== inputHash) {
          this.logger?.debug(`Input hash mismatch for task ${taskUniqueId}`);
          throw new IdempotencyConflictException();
        }
        this.logger?.debug(`Deserialized result: ${deserializedResult}`);
        return deserializedResult.executionResult;
      }

      const result = await callbackFn(...input);

      this.logger?.debug(`Execution was sucessful, saving result to storage`);
      const idempotentResult: IdempotencyResult<ReturnType<F>> = {
        executionResult: result,
        executionInputHash: inputHash,
      };

      const serializedIdempotentResult = await this.serializer.serialize(idempotentResult);

      this.logger?.debug(`Serialized result: ${serializedIdempotentResult}`);

      const compressedIdempotentResult = await this.compressIfEnabled(
        !!options?.shouldCompress,
        serializedIdempotentResult
      );

      this.logger?.debug(`Compressed result: ${compressedIdempotentResult}`);

      await this.storage.save(taskUniqueId, compressedIdempotentResult, {
        ttl: ttl ?? null,
        context: {
          taskName: callbackFn.name,
        },
      });
      this.logger?.debug(`Saved result for task ${taskUniqueId}`);
      return result;
    };
}
