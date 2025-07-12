import { IdempotentLogger } from '../base';

export class ConsoleLogger implements IdempotentLogger {
  error(message: string): void {
    console.error(message);
  }

  debug(message: string): void {
    console.debug(message);
  }
}
