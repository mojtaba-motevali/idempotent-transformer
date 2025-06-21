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

export abstract class IdempotentStateStore {
  abstract connect(): Promise<void> | void;
  abstract disconnect(): Promise<void> | void;
  abstract isConnected(): Promise<boolean> | boolean;
  abstract find(id: string): Promise<TSerialized | null> | TSerialized | null;
  abstract save(id: string, value: TSerialized, options: IStateStoreOptions): Promise<void> | void;
}
