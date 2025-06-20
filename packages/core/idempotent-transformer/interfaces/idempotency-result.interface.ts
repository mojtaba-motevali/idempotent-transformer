export interface IdempotencyResult<T> {
  re: T;
  in: string;
}
