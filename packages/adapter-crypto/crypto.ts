import { IdempotentCrypto } from '@idempotent-transformer/base/crypto';
import { createHash } from 'crypto';
import { TSerialized } from '@idempotent-transformer/base/types/serialized.type';

export class Md5Adapter extends IdempotentCrypto {
  async createHash(value: TSerialized): Promise<string> {
    return createHash('md5').update(value).digest('hex');
  }
}
