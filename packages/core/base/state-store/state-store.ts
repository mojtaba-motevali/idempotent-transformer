import { TSerialized } from '../types/serialized.type';

export interface IIdempotentStateStoreOptions {
  /**
   * Context of the task that was executed.
   */
  context: {
    taskName: string;
  };

  /**
   * The workflow id.
   */
  workflowId: string;
}

export interface IdempotentStateStore {
  connect(): Promise<void> | void;
  disconnect(): Promise<void> | void;
  isConnected(): Promise<boolean> | boolean;
  /**
   * Find a task by workflowId and taskId.
   * @param workflowId - The workflow id.
   * @param taskId - The task id.
   * @returns {TSerialized} - The stored workflowId/taskId/value pair.
   */
  find({
    workflowId,
    taskId,
  }: {
    workflowId: string;
    taskId: string;
  }):
    | Promise<{ workflowId: string; taskId: string; value: TSerialized } | null>
    | null
    | { workflowId: string; taskId: string; value: TSerialized };
  /**
   * Find all the tasks by workflowId.
   * @param workflowId - The workflow id.
   * @returns {TSerialized[]} - The stored workflowId/taskId/value pairs.
   */
  findAll(
    workflowId: string
  ):
    | Promise<{ workflowId: string; taskId: string; value: TSerialized }[]>
    | { workflowId: string; taskId: string; value: TSerialized }[];
  /**
   * Save workflowId, taskId and value to the state store.
   * @param workflowId - The workflow id.
   * @param taskId - The task id.
   * @param value - The value to save.
   * @param options - The options for the state store.
   */
  save(
    { workflowId, taskId, value }: { workflowId: string; taskId: string; value: TSerialized },
    options: IIdempotentStateStoreOptions
  ): Promise<void> | void;
  /**
   * Complete a workflow.
   * @param workflowId - The workflow id.
   * @param expiredAt - The date after which the workflow should be expired.
   */
  complete(workflowId: string, expiredAt: number): Promise<void> | void;

  /**
   * Clean all expired workflows.
   */
  cleanExpired(): Promise<void> | void;
}
