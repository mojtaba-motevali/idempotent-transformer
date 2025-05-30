export interface IdempotencyResult<T> {
  executionResult: T;
  executionInputHash: string;
}
