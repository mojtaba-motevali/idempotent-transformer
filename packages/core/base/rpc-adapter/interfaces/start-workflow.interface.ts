export interface StartWorkflowInput {
  workflowId: string;
  name: string;
}

export interface StartWorkflowOutput {
  fencingToken: number;
}
