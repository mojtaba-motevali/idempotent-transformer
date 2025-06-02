import { Compressor } from '../base/compressor';
import { Logger } from '../base/logger';
import { Serializer } from '../base/serializer';
import { StateStore } from '../base/state-store';
import { TSerialized } from '../base/types/serialized.type';
import { IdempotencyConflictException } from './exceptions/conflict.exception';
import { IdempotencyResult } from './interfaces/idempotency-result.interface';
import { IdempotencyKey } from './interfaces/idempotent-key.interface';
import { Options } from './interfaces/idempotent-options.interface';
import { IdempotentTransformerInput } from './interfaces/idempotent-transformer.interface';
import { MakeIdempotentResult } from './interfaces/make-idempotent.interface';
import crypto from 'node:crypto';

export class IdempotentTransformer implements IdempotentTransformer {
  private static instance: IdempotentTransformer;
  private storage: StateStore;
  private serializer: Serializer;
  private compressor: Compressor;
  private logger: Logger;

  private constructor(input: IdempotentTransformerInput) {
    const { storage, serializer, compressor, log } = input;
    this.storage = storage;
    this.serializer = serializer;
    this.compressor = compressor;
    this.logger = log;
  }

  static getInstance(input: IdempotentTransformerInput) {
    if (!IdempotentTransformer.instance) {
      const { storage, serializer, compressor, log } = input;
      IdempotentTransformer.instance = new IdempotentTransformer({
        storage,
        serializer,
        compressor,
        log,
      });
    }
    return IdempotentTransformer.instance;
  }

  makeIdempotent<T extends Record<string, (input: any) => Promise<any>>>(
    workflowId: string,
    functions: T
  ): MakeIdempotentResult<T> {
    const callbacks = functions as Record<string, (input: any) => Promise<any>>;

    const result = {} as MakeIdempotentResult<T>;

    for (const key of Object.keys(callbacks)) {
      result[key as keyof T] = this.createTransformer(workflowId, callbacks[key]);
    }

    return result;
  }

  private async compressIfEnabled(
    shouldCompress: boolean,
    value: TSerialized
  ): Promise<TSerialized | Uint8Array<ArrayBufferLike>> {
    if (shouldCompress) {
      return this.compressor.compress(value);
    }
    return value;
  }

  private async decompressIfCompressed(value: Uint8Array<ArrayBufferLike>): Promise<TSerialized> {
    if (this.compressor.isCompressed(value)) {
      return this.compressor.decompress(value);
    }
    return value;
  }

  async createHash<T>(value: T): Promise<string> {
    return crypto
      .createHash('md5')
      .update(await this.serializer.serialize(value))
      .digest('hex');
  }

  /**
   * Creates a new idempotent transformer
   * @param transformFn The transformation function
   * @returns An IdempotentTransformer instance
   */
  private createTransformer =
    <T>(workflowId: string, callbackFn: (input: T) => Promise<T>) =>
    async (input: T, idempotencyKey: IdempotencyKey, options?: Options): Promise<T> => {
      this.logger.debug(`Creating transformer for workflow ${workflowId}`);
      /**
       * The task unique id is a hash of the workflow id and the idempotency key.
       */
      const taskUniqueId = await this.createHash({
        workflowId,
        idempotencyKey,
      });
      /**
       * The input hash is a hash of the input.
       */
      const inputHash = await this.createHash(input);

      this.logger.debug(`Task unique id: ${taskUniqueId}`);
      const cachedResult = await this.storage.find(taskUniqueId);
      this.logger.debug(`Cached result: ${cachedResult}`);
      if (cachedResult) {
        this.logger.debug(`Found cached result for task ${taskUniqueId}`);
        const decompressedResult = await this.decompressIfCompressed(cachedResult);
        const deserializedResult =
          await this.serializer.deserialize<IdempotencyResult<T>>(decompressedResult);
        if (deserializedResult.executionInputHash !== inputHash) {
          this.logger.debug(`Input hash mismatch for task ${taskUniqueId}`);
          throw new IdempotencyConflictException();
        }
        this.logger.debug(`Deserialized result: ${deserializedResult}`);
        return deserializedResult.executionResult;
      }

      const result = await callbackFn(input);

      this.logger.debug(`Execution was sucessful, saving result to storage`);
      const idempotentResult: IdempotencyResult<T> = {
        executionResult: result,
        executionInputHash: inputHash,
      };
      const serializedIdempotentResult = await this.serializer.serialize(idempotentResult);

      this.logger.debug(`Serialized result: ${serializedIdempotentResult}`);

      const compressedIdempotentResult = await this.compressIfEnabled(
        !!options?.shouldCompress,
        serializedIdempotentResult
      );

      this.logger.debug(`Compressed result: ${compressedIdempotentResult}`);

      await this.storage.save(taskUniqueId, compressedIdempotentResult, {
        methodName: callbackFn.name,
        ...(options ? options : {}),
      });
      this.logger.debug(`Saved result for task ${taskUniqueId}`);
      return result;
    };
}
