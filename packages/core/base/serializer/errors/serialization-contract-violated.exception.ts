export class SerializationContractViolatedException extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'SerializationContractViolatedException';
  }
}
