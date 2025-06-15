import { createClient, RedisClientOptions } from 'redis';
import { IStateStoreOptions, IdempotentStateStore } from '../../lib/base/state-store';

export class RedisAdapter extends IdempotentStateStore {
  private client: ReturnType<typeof createClient>;
  constructor(url: string, options?: RedisClientOptions) {
    super();
    this.client = createClient({ url, ...(options ? options : {}) });
  }

  async connect() {
    await this.client.connect();
    this.client.on('error', (err) => {
      console.error('Redis error:', err);
    });
  }

  async disconnect() {
    await this.client.quit();
  }

  async isConnected() {
    return this.client.isOpen;
  }

  /**
   * Get the expiry command for the Redis SET command.
   * @param ttl - The TTL in milliseconds.
   * @returns The expiry command for the Redis SET command.
   */
  private getExpiry(ttl: number | null): string[] {
    if (ttl) {
      return ['PX', ttl.toString()];
    }
    return [];
  }

  save = async (
    key: string,
    value: Uint8Array<ArrayBufferLike>,
    options: IStateStoreOptions
  ): Promise<void> => {
    const buffer = Buffer.from(value);
    await this.client.sendCommand(['SET', key, buffer, ...this.getExpiry(options.ttl)], {
      typeMapping: {
        '36': Buffer,
      },
    });
  };

  find = async (key: string): Promise<Uint8Array<ArrayBufferLike> | null> => {
    const value: Buffer | null = await this.client.sendCommand(['GET', key], {
      // By default, the value is returned as a string, so we need to map it to a Buffer to avoid data loss.
      typeMapping: {
        '36': Buffer,
      },
    });
    return value ? new Uint8Array(value) : null;
  };
}
