export class LeaseTimeoutNotFoundException extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'lease_timeout_not_found';
  }
}
