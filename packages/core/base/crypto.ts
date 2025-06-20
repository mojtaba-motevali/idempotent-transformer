import { TSerialized } from './types/serialized.type';

export abstract class IdempotentCrypto {
  abstract createHash<T extends TSerialized>(value: T): Promise<string>;
}
