export interface IdempotentLogger {
  error(message: string): void;
  debug(message: string): void;
}
