import { Options } from './idempotent-options.interface';

export interface IdempotencyResult<T> {
  executionResult: T;
  executionInputHash: string;
}
