import { Logger } from '../base/logger';

export class ConsoleLogger extends Logger {
  error(message: string): void {
    console.error(message);
  }

  debug(message: string): void {
    console.debug(message);
  }
}
