export interface GenerateIdempotencyKeyInput {
  workflowId: string;
  fencingToken: number;
  position: number;
}

export interface GenerateIdempotencyKeyOutput {
  idempotencyKey: string;
}
