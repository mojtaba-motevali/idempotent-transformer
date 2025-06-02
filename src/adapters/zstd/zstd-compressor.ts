import { zstdCompress, zstdDecompress } from 'node:zlib';
import { Compressor } from '../../lib/base/compressor';
import { TBinary } from '../../lib/base/types/binary.type';

export class ZstdCompressor extends Compressor {
  private readonly magic = Buffer.from([0x28, 0xb5, 0x2f, 0xfd]);
  constructor() {
    super();
  }

  compress = async (data: TBinary): Promise<Buffer<ArrayBufferLike>> => {
    return new Promise((resolve, reject) => {
      zstdCompress(data, (err, result) => {
        if (err) {
          reject(err);
        }
        resolve(result);
      });
    });
  };

  decompress = async (data: TBinary): Promise<Buffer<ArrayBufferLike>> => {
    return new Promise((resolve, reject) => {
      zstdDecompress(data, (err, result) => {
        if (err) {
          reject(err);
        }
        resolve(result);
      });
    });
  };

  isCompressed = (data: TBinary): boolean => {
    return data.slice(0, 4).every((value, index) => value === this.magic[index]);
  };
}
