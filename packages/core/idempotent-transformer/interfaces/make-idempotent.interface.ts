import { WorkflowStatusInput, WorkflowStatusOutput } from '../../base';
import { IIdempotentTaskOptions } from './idempotent-options.interface';

export interface MakeIdempotentInput<T extends Record<string, (input: any) => any>> {
  functions: T;
}

export type IdempotentRunnerResult = {
  complete: () => Promise<void>;
  execute: <R>(
    idempotencyKey: string,
    task: (() => Promise<R>) | (() => R),
    options?: IIdempotentTaskOptions<R>
  ) => Promise<R>;
  getWorkflowStatus: (args: WorkflowStatusInput) => Promise<WorkflowStatusOutput>;
};

export interface IdempotentTransformerOptions {
  /**
   * The number of milliseconds when the task result will be considered expired.
   * @default null
   */
  completedRetentionTime?: number | null;

  /**
   * The context name to use for the workflow. This is used to differentiate nested workflows.
   */
  workflowName: string;
}
