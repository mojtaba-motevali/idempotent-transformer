export interface LeaseWorkflowInput {
  workflow_id: string;
  is_nested: boolean;
  prefetch_checkpoints: boolean;
  context_name: string;
}

export interface LeaseWorkflowOutput {
  checkpoints: Record<string, Uint8Array>;
  fencing_token: number;
}
