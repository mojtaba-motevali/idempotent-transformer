export abstract class IdempotentLogger {
  abstract error(message: string): void;
  abstract debug(message: string): void;
}
