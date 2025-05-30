import { decode, Decoder, Encoder, ExtensionCodec } from '@msgpack/msgpack';
import { Serializer } from '../../lib/base/serializer';
import { TSerialized } from '../../lib/base/types/serialized.type';
import { MessagePackOptions } from './message-pack.interface';

export class MessagePack extends Serializer {
  private encoder: Encoder;
  private decoder: Decoder;
  private extensionCodec: ExtensionCodec;

  constructor({ models }: MessagePackOptions = {}) {
    super();
    this.extensionCodec = new ExtensionCodec();
    this.encoder = new Encoder({ extensionCodec: this.extensionCodec, ignoreUndefined: true });
    this.decoder = new Decoder({ extensionCodec: this.extensionCodec });
    this.build(models);
  }

  build(models: MessagePackOptions['models']) {
    // Set<T>
    this.extensionCodec.register({
      type: 0,
      encode: (object: unknown): Uint8Array | null => {
        if (object instanceof Set) {
          return this.encoder.encode([...object]);
        } else {
          return null;
        }
      },
      decode: (data: Uint8Array) => {
        const array = decode(data) as Array<unknown>;
        return new Set(array);
      },
    });
    this.extensionCodec.register({
      type: 1,
      encode: (object: unknown): Uint8Array | null => {
        if (object instanceof Map) {
          return this.encoder.encode([...object]);
        } else {
          return null;
        }
      },
      decode: (data: Uint8Array) => {
        const array = decode(data) as Array<[unknown, unknown]>;
        return new Map(array);
      },
    });

    if (!models) {
      return;
    }

    this.extensionCodec.register({
      type: 2,
      encode: (object: unknown): Uint8Array | null => {
        if (object instanceof Object) {
          for (const { name, model } of models) {
            if (object instanceof model) {
              return this.encoder.encode([name, object.toJSON()]);
            }
          }
          return null;
        }
        return null;
      },
      decode: (data: Uint8Array) => {
        const array = decode(data) as [string, unknown];
        const [name, object] = array;
        const model = models.find(({ name: modelName }) => modelName === name);
        if (model) {
          return new model.model(object);
        }
        return object;
      },
    });
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
