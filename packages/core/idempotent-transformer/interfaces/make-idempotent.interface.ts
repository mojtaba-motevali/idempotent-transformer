import { IIdempotentTaskOptions } from './idempotent-options.interface';

export interface MakeIdempotentInput<T extends Record<string, (input: any) => any>> {
  functions: T;
}

export type IdempotentRunnerResult = {
  complete: () => Promise<void>;
  execute: (
    idempotencyKey: string,
    fn: (...args: any[]) => Promise<any>,
    options?: IIdempotentTaskOptions
  ) => Promise<any>;
};

export interface IdempotentTransformerOptions {
  /**
   * The number of milliseconds when the task result will be considered expired.
   * @default null
   */
  retentionTime?: number | null;

  /**
   * Whether to prefetch checkpoints. Use this when number of tasks and their result are known to you.
   * There is a trade off between memory usage and performance.
   * @default false
   */
  prefetchCheckpoints?: boolean;

  /**
   * The context name to use for the workflow. This is used to differentiate nested workflows.
   */
  contextName: string;

  /**
   * The workflow id to use for the workflow.
   * @default false
   */
  isNested?: boolean;
}
