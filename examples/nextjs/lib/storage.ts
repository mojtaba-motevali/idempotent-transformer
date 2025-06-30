import { IdempotentStateStore, TSerialized } from '@idempotent-transformer/core';

export class InMemoryStorage extends IdempotentStateStore {
  private dataStateMap: Map<string, TSerialized>;
  constructor() {
    super();
    this.dataStateMap = new Map();
  }
  async connect(): Promise<void> {}
  async disconnect(): Promise<void> {}
  async isConnected(): Promise<boolean> {
    return true;
  }
  async find(key: string): Promise<TSerialized | null> {
    return this.dataStateMap.get(key) || null;
  }
  async save(key: string, value: TSerialized): Promise<void> {
    this.dataStateMap.set(key, value);
  }
}
