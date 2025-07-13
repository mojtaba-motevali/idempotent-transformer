import { IdempotentCheckSumGenerator, TSerialized } from '@idempotent-transformer/core';
import { crc32 } from 'node:zlib';

export class CheckSumGenerator implements IdempotentCheckSumGenerator {
  constructor() {}

  async generate<T extends TSerialized>(value: T): Promise<string | number> {
    return crc32(value);
  }
}
