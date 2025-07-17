export class CheckpointLeasedByOtherWorkerException extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'checkpoint_leased_by_other_worker';
  }
}
