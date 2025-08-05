export interface LeaseCheckpointInput {
  workflowId: string;
  fencingToken: number;
  leaseTimeout: number;
  position: number;
  idempotencyKey: string;
}

export interface LeaseCheckpointOutput {
  value?: Buffer;
  remainingLeaseTimeout?: number;
}
