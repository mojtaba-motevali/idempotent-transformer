import { TBinary } from './types/binary.type';

export abstract class Compressor {
  abstract compress: (data: TBinary) => Promise<Buffer<ArrayBufferLike>>;
  abstract decompress: (data: TBinary) => Promise<Buffer<ArrayBufferLike>>;
  abstract isCompressed: (data: TBinary) => boolean;
}
