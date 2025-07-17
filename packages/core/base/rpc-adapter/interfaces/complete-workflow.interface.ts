export interface CompleteWorkflowInput {
  workflow_id: string;
  fencing_token: number;
  expire_after: number;
}

export interface CompleteWorkflowOutput {}
