import { TSerialized } from '../../types/serialized.type';

export interface CheckpointInput {
  workflowId: string;
  value: TSerialized;
  fencingToken: number;
  position: number;
  idempotencyKey: string;
}

export interface CheckpointOutput {
  abort: boolean;
}
