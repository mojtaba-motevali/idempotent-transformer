export interface LeaseCheckpointInput {
  workflowId: string;
  fencingToken: number;
  leaseTimeout: number;
  positionChecksum: number;
  idempotencyChecksum: number;
}

export interface LeaseCheckpointOutput {
  value?: Buffer;
}
