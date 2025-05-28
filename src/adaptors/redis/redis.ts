export interface RedisClient {
  get(key: string): Promise<string | null>;
  set(key: string, value: string): Promise<void>;
  connect(): Promise<void>;
  disconnect(): Promise<void>;
  createClient(url: string): RedisClient;
}
