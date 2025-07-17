export class NestedWorkflowFencingTokenConflictException extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'nested_workflow_fencing_token_conflict';
  }
}
