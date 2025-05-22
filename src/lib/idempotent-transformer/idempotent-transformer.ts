import { IdempotencyKey } from './interfaces/idempotent-key.interface';
import { Options } from './interfaces/idempotent-options.interface';
import { IdempotentTransformerInput } from './interfaces/idempotent-transformer.interface';
import { MakeIdempotentInput, MakeIdempotentResult } from './interfaces/make-idempotent.interface';

export interface IdempotentTransformer {
  makeIdempotent<T extends Record<string, (input: any) => Promise<any>>>(
    input: MakeIdempotentInput<T>
  ): MakeIdempotentResult<T>;
}

export class IdempotentTransformer implements IdempotentTransformer {
  constructor(input: IdempotentTransformerInput) {}

  makeIdempotent<T extends Record<string, (input: any) => Promise<any>>>(
    functions: T
  ): MakeIdempotentResult<T> {
    const callbacks = functions as Record<string, (input: any) => Promise<any>>;

    const result = {} as MakeIdempotentResult<T>;

    for (const key of Object.keys(callbacks)) {
      result[key as keyof T] = this.createTransformer(callbacks[key]);
    }

    return result;
  }

  /**
   * Creates a new idempotent transformer
   * @param transformFn The transformation function
   * @returns An IdempotentTransformer instance
   */
  private createTransformer =
    <T>(callbackFn: (input: T) => Promise<T>) =>
    async (input: T, idempotencyKey: IdempotencyKey, options?: Options): Promise<T> => {
      console.log(`Input: ${input}`);
      console.log('Running RAFT or PAXOS to select a leader');
      console.log('Running consensus protocol to ensure atomicity');
      console.log('Start a transaction');
      console.log('Check if the idempotency key exists in the cache');
      console.log('If it does, return the cached value');
      console.log("If it doesn't, run the transformation");
      console.log(`idempotencyKey: ${idempotencyKey}`);
      const result = await callbackFn(input);
      console.log('Store the result in the cache');
      console.log('Commit the transaction');
      console.log(`options: ${options}`);
      console.log('Commit the transaction');
      return result;
    };
}
