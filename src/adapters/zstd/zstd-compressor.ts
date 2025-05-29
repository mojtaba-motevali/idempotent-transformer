import { zstdCompress, zstdDecompress } from 'node:zlib';
import { Compressor } from '../../lib/base/compressor';
import { TBinary } from '../../lib/base/types/binary.type';

export class ZstdCompressor extends Compressor {
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
}
