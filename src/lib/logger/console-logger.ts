import { Logger } from '../base/logger';

export class ConsoleLogger extends Logger {
  error(message: string): void {}

  debug(message: string): void {}
}
