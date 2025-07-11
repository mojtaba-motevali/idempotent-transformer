import { TSerialized } from './types/serialized.type';

export interface IStateStoreOptions {
  /**
   * The number of milliseconds when the task result will be considered expired.
   */
  ttl: number | null;
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
  find(id: string): Promise<TSerialized | null> | TSerialized | null;
  save(id: string, value: TSerialized, options: IStateStoreOptions): Promise<void> | void;
  complete(workflowId: string): Promise<void> | void;
  cleanAll(beforeDate: Date): Promise<void> | void;
}
