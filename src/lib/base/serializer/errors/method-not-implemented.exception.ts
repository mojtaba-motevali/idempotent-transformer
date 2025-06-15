export class MethodNotImplementedException extends Error {
  constructor(methodName: string) {
    super(`${methodName} is not implemented, please implement it.`);
    this.name = 'MethodNotImplementedException';
  }
}
