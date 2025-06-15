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
}

export abstract class IdempotentStateStore {
  abstract connect(): Promise<void>;
  abstract disconnect(): Promise<void>;
  abstract isConnected(): Promise<boolean>;
  abstract find(id: string): Promise<TSerialized | null>;
  abstract save(id: string, value: TSerialized, options: IStateStoreOptions): Promise<void>;
}
