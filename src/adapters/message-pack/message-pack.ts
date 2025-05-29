import { Decoder, Encoder } from '@msgpack/msgpack';
import { Serializer } from '../../lib/base/serializer';
import { TSerialized } from '../../lib/base/types/serialized.type';

export class MessagePack extends Serializer {
  private encoder: Encoder;
  private decoder: Decoder;

  constructor() {
    super();
    this.encoder = new Encoder();
    this.decoder = new Decoder();
  }

  serialize = async <T>(data: T): Promise<TSerialized> => {
    const encoded = this.encoder.encode(data);
    return encoded;
  };

  deserialize = async <T>(data: TSerialized): Promise<T> => {
    let buffer: Uint8Array;

    if (data instanceof Blob) {
      const arrayBuffer = await data.arrayBuffer();
      buffer = new Uint8Array(arrayBuffer);
    } else if (data instanceof Uint8Array) {
      buffer = data;
    } else {
      throw new Error('Input data must be either a Blob or Uint8Array');
    }

    const decoded = this.decoder.decode(buffer);
    return decoded as T;
  };
}
