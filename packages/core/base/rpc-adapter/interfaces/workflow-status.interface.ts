export interface WorkflowStatusInput {
  workflowId: string;
}

export interface WorkflowStatusOutput {
  id: string;
  status: number;
  expireAt?: number;
  completedAt?: number;
}
