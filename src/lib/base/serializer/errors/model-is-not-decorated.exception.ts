export class ModelIsNotDecoratedException extends Error {
  constructor(modelName: string) {
    super(`${modelName} is not decorated with @Serialize, please decorate it.`);
    this.name = 'ModelIsNotDecoratedException';
  }
}
