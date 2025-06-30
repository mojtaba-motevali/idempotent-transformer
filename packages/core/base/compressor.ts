import { TSerialized } from './types/serialized.type';

export abstract class IdempotentCompressor {
  abstract compress(data: TSerialized): Promise<TSerialized>;
  abstract decompress(data: TSerialized): Promise<TSerialized>;
  abstract isCompressed(data: TSerialized): boolean;
}
