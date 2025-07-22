import { TSerialized } from '../../types/serialized.type';

export interface CheckpointInput {
  workflowId: string;
  value: TSerialized;
  fencingToken: number;
  positionChecksum: number;
  idempotencyChecksum: number;
}

export interface CheckpointOutput {
  abort: boolean;
}
