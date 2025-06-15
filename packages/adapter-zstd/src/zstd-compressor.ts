import { zstdCompress, zstdDecompress } from 'node:zlib';
import { IdempotentCompressor } from '@idempotent-transformer/base';
import { TSerialized } from '@idempotent-transformer/base';

export class ZstdCompressor extends IdempotentCompressor {
  private readonly magic = Buffer.from([0x28, 0xb5, 0x2f, 0xfd]);
  constructor() {
    super();
  }

  compress = async (data: TSerialized): Promise<TSerialized> => {
    return new Promise((resolve, reject) => {
      zstdCompress(data, (err, result) => {
        if (err) {
          reject(err);
        }
        resolve(result);
      });
    });
  };

  decompress = async (data: TSerialized): Promise<TSerialized> => {
    return new Promise((resolve, reject) => {
      zstdDecompress(data, (err, result) => {
        if (err) {
          reject(err);
        }
        resolve(result);
      });
    });
  };

  isCompressed = (data: TSerialized): boolean => {
    return (
      data instanceof Uint8Array &&
      data.slice(0, 4).every((value, index) => value === this.magic[index])
    );
  };
}
