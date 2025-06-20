import { IdempotentCrypto, TSerialized } from '@idempotent-transformer/core';
import { createHash } from 'crypto';

export class Md5Adapter extends IdempotentCrypto {
  async createHash(value: TSerialized): Promise<string> {
    return createHash('md5').update(value).digest('hex');
  }
}
