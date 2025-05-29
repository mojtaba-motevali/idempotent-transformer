import { createClient, RedisClientOptions } from 'redis';
import { StateStore } from '../../lib/base/state-store';

export class Repository extends StateStore {
  private client: ReturnType<typeof createClient>;
  constructor(url: string, options?: RedisClientOptions) {
    super();
    this.client = createClient({ url, ...(options ? options : {}) });
  }

  async initialize() {
    await this.client.connect();
    this.client.on('error', (err) => {
      console.error('Redis error:', err);
    });
  }

  save = async (key: string, value: Uint8Array<ArrayBufferLike>): Promise<void> => {
    const buffer = Buffer.from(value);
    await this.client.sendCommand(['SET', key, buffer], {
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
