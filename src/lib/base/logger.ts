export abstract class Logger {
  abstract error(message: string): void;
  abstract debug(message: string): void;
}
