import { IOptions } from './idempotent-options.interface';

export interface MakeIdempotentInput<T extends Record<string, (input: any) => any>> {
  functions: T;
}

export type MakeIdempotentResult<T extends Record<string, (input: any) => Promise<any>>> = {
  [K in keyof T]: (...args: [...Parameters<T[K]>, IOptions?]) => Promise<ReturnType<T[K]>>;
};

export interface IdempotentTransformerOptions {
  /**
   * The number of milliseconds when the task result will be considered expired.
   * @default 1 hour
   */
  ttl?: number | null;
}
