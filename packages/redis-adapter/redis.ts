import { Redis, RedisOptions } from 'ioredis';
import { IdempotentStateStore } from '@idempotent-transformer/core';

export class RedisAdapter implements IdempotentStateStore {
  private basePath: string;
  private client: Redis;
  constructor({ options, redis }: { options?: RedisOptions; redis?: Redis }) {
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

  async save({
    workflowId,
    taskId,
    value,
  }: {
    workflowId: string;
    taskId: string;
    value: Uint8Array<ArrayBufferLike>;
  }): Promise<void> {
    const buffer = Buffer.from(value);
    await this.client.set(`${this.basePath}:${workflowId}:${taskId}`, buffer, 'GET');
  }

  async findAll(
    workflowId: string
  ): Promise<{ workflowId: string; taskId: string; value: Uint8Array<ArrayBufferLike> }[]> {
    const luaScript = `
      local pattern = ARGV[1]
      local keys = redis.call('KEYS', pattern)
      if #keys == 0 then
        return {}
      end
      local result = {}
      local values = redis.call('MGET', unpack(keys))
      for i, key in ipairs(keys) do
        table.insert(result, key)
        table.insert(result, values[i])
      end
      return result
    `;

    const raw = (await this.client.eval(luaScript, 0, `${this.basePath}:${workflowId}:*`)) as (
      | string
      | null
    )[];

    const output: { workflowId: string; taskId: string; value: Uint8Array<ArrayBufferLike> }[] = [];

    for (let i = 0; i < raw.length; i += 2) {
      const key = raw[i];
      const val = raw[i + 1];
      if (!key || val == null) continue;
      const taskId = key.toString().split(':').pop() ?? '';
      const buffer = Buffer.isBuffer(val) ? (val as Buffer) : Buffer.from(val, 'binary');
      output.push({ workflowId, taskId, value: new Uint8Array(buffer) });
    }

    return output;
  }

  async find({
    workflowId,
    taskId,
  }: {
    workflowId: string;
    taskId: string;
  }): Promise<{ workflowId: string; taskId: string; value: Uint8Array<ArrayBufferLike> } | null> {
    const value: Buffer | null = await this.client.getBuffer(
      `${this.basePath}:${workflowId}:${taskId}`
    );
    return value ? { workflowId, taskId, value: new Uint8Array(value) } : null;
  }

  async clean(workflowId: string): Promise<void> {
    await this.client.del(`${this.basePath}:${workflowId}`);
  }

  async complete(workflowId: string): Promise<void> {
    // Lua script to get keys matching pattern and set expiration within Redis
    const luaScript = `
      local pattern = KEYS[1]
      local ttl = tonumber(ARGV[1])
      local keys = redis.call('KEYS', pattern)
      local count = 0
      for i, key in ipairs(keys) do
        redis.call('EXPIRE', key, ttl)
        count = count + 1
      end
      return count
    `;

    await this.client.eval(luaScript, 1, `${this.basePath}:${workflowId}*`, '86400');
  }

  async cleanAll(beforeDate: Date): Promise<void> {
    const keys = await this.client.keys(`${this.basePath}:*`);
    await this.client.del(keys);
  }

  async cleanExpired(): Promise<void> {
    // Redis handles key expiration automatically via TTL, so nothing to do here.
  }
}
