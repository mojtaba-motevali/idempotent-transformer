export class IdempotencyConflictException extends Error {
  constructor() {
    super('The task execution failed because it was called with different parameters.');
    this.name = 'IdempotencyConflictException';
  }
}
