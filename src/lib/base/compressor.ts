import { TSerializable } from './types/serializable.type';

export abstract class Compressor {
  abstract compress: (data: TSerializable) => Promise<BinaryType>;
  abstract decompress: (data: BinaryType) => Promise<TSerializable>;
}
