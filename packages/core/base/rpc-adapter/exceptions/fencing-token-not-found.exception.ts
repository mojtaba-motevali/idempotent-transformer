export class FencingTokenNotFoundException extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'fencing_token_not_found';
  }
}
