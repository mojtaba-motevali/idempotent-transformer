import { TSerialized } from './types/serialized.type';

export interface IdempotentCrypto {
  createHash<T extends TSerialized>(value: T): Promise<string>;
}
