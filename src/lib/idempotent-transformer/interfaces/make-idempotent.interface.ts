import { IdempotencyKey } from './idempotent-key.interface';
import { Options } from './idempotent-options.interface';

export interface MakeIdempotentInput<T extends Record<string, (input: any) => any>> {
  functions: T;
}

export type MakeIdempotentResult<T extends Record<string, (input: any) => Promise<any>>> = {
  [K in keyof T]: (
    input: Parameters<T[K]>[0],
    idempotencyKey: IdempotencyKey,
    options?: Options
  ) => Promise<ReturnType<T[K]>>;
};
