export interface LeaseCheckpointInput {
  workflow_id: string;
  fencing_token: number;
  lease_timeout: number;
  position_checksum: number;
}

export interface LeaseCheckpointOutput {
  value?: Buffer;
}
