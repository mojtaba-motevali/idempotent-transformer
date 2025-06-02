import { IdempotencyKey } from './idempotent-key.interface';
import { IOptions } from './idempotent-options.interface';

export interface MakeIdempotentInput<T extends Record<string, (input: any) => any>> {
  functions: T;
}

export type MakeIdempotentResult<T extends Record<string, (input: any) => Promise<any>>> = {
  [K in keyof T]: (
    input: Parameters<T[K]>[0],
    idempotencyKey: IdempotencyKey,
    options?: IOptions
  ) => Promise<ReturnType<T[K]>>;
};

export interface IdempotentTransformerOptions {
  /**
   * The number of milliseconds when the task result will be considered expired.
   * @default 1 hour
   */
  ttl?: number | null;
}
