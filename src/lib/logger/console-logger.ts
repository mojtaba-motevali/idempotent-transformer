import { IdempotentLogger } from '../base/logger';

export class ConsoleLogger extends IdempotentLogger {
  error(message: string): void {}

  debug(message: string): void {}
}
