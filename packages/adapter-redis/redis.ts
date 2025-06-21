import { Redis, RedisOptions } from 'ioredis';
import { IStateStoreOptions, IdempotentStateStore } from '@idempotent-transformer/core';

export class RedisAdapter extends IdempotentStateStore {
  private basePath: string;
  private client: Redis;
  constructor({ options, redis }: { options?: RedisOptions; redis?: Redis }) {
    super();
    if (!options && !redis) {
      throw new Error('Redis client is not provided');
    }
    this.client =
      redis ??
      new Redis({
        ...options,
        lazyConnect: true,
        enableReadyCheck: true,
      });
    this.basePath = 'idempotent-workflows';
  }

  async connect() {
    if (this.isConnected()) {
      return;
    }
    await this.client.connect();
    this.client.on('error', (err: Error) => {
      console.error('Redis error:', err);
    });
  }

  async disconnect() {
    if (!this.isConnected()) {
      return;
    }
    await this.client.quit();
  }

  isConnected() {
    return (
      this.client.status === 'ready' ||
      this.client.status === 'connecting' ||
      this.client.status === 'connect'
    );
  }

  save = async (
    key: string,
    value: Uint8Array<ArrayBufferLike>,
    options: IStateStoreOptions
  ): Promise<void> => {
    const buffer = Buffer.from(value);
    if (options.ttl) {
      await this.client.set(`${this.basePath}:${key}`, buffer, 'PX', options.ttl.toString());
    } else {
      await this.client.set(`${this.basePath}:${key}`, buffer, 'GET');
    }
  };

  find = async (key: string): Promise<Uint8Array<ArrayBufferLike> | null> => {
    const value: Buffer | null = await this.client.getBuffer(`${this.basePath}:${key}`);
    return value ? new Uint8Array(value) : null;
  };
}
