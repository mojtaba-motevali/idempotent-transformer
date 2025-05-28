import { TBinary } from './types/binary.type';

export abstract class Compressor {
  abstract compress: (data: TBinary) => Promise<unknown>;
  abstract decompress: (data: unknown) => Promise<TBinary>;
}
