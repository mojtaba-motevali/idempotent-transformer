import { IIdempotentTaskOptions } from './idempotent-options.interface';

export interface MakeIdempotentInput<T extends Record<string, (input: any) => any>> {
  functions: T;
}

export type MakeIdempotentResult<T extends Record<string, (input: any) => Promise<any> | any>> = {
  [K in keyof T]: (
    ...args: [...Parameters<T[K]>, IIdempotentTaskOptions?]
  ) => Promise<ReturnType<T[K]>>;
} & {
  complete: () => Promise<void>;
};

export interface IdempotentTransformerOptions {
  /**
   * The number of milliseconds when the task result will be considered expired.
   * @default null
   */
  ttl?: number | null;
}
