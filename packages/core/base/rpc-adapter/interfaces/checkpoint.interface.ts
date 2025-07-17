import { TSerialized } from '../../types/serialized.type';

export interface CheckpointInput {
  workflow_id: string;
  value: TSerialized;
  fencing_token: number;
  position_checksum: number;
  workflow_context_name: string;
  checkpoint_context_name: string;
}

export interface CheckpointOutput {
  abort: boolean;
}
