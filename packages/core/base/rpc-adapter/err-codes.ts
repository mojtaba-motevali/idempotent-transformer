export enum ErrCodes {
  CheckpointLeasedByOtherWorker = 'checkpoint_leased_by_other_worker',
  FencingTokenExpired = 'fencing_token_expired',
  FencingTokenNotFound = 'fencing_token_not_found',
  NonDeterministicCheckpointFound = 'non_deterministic_checkpoint_found',
}
