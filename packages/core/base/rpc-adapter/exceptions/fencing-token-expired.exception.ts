export class FencingTokenExpiredException extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'fencing_token_expired';
  }
}
