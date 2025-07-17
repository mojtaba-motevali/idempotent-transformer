export class WorkflowAbortedException extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'workflow_aborted';
    this.message = message;
  }
}
