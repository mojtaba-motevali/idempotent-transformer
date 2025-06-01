import { decode, Decoder, Encoder, ExtensionCodec } from '@msgpack/msgpack';
import { Serializer } from '../../lib/base/serializer';
import { TSerialized } from '../../lib/base/types/serialized.type';
import { Serializable } from './message-pack.interface';
import { decoratedModels, SERIALIZE_NAME_METADATA_KEY } from './decorators/serialize.decorator';
import { ModelIsNotDecoratedException } from './errors/model-is-not-decorated.exception';
import { throwIfTrue } from './lib/throw-if-true';
import { MethodNotImplementedException } from './errors/method-not-implemented.exception';

export class MessagePack extends Serializer {
  private encoder: Encoder;
  private decoder: Decoder;
  private extensionCodec: ExtensionCodec;

  private static instance: MessagePack;

  private constructor() {
    super();
    this.extensionCodec = new ExtensionCodec();
    this.encoder = new Encoder({ extensionCodec: this.extensionCodec, ignoreUndefined: true });
    this.decoder = new Decoder({ extensionCodec: this.extensionCodec });
    this.build();
  }

  static getInstance() {
    if (!MessagePack.instance) {
      MessagePack.instance = new MessagePack();
    }
    return MessagePack.instance;
  }

  build() {
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

    this.extensionCodec.register({
      type: 2,
      encode: (object: unknown): Uint8Array | null => {
        if (object instanceof Object) {
          const unBoxedObject = object as unknown as Serializable;
          const key: string = (unBoxedObject as any).constructor[SERIALIZE_NAME_METADATA_KEY];
          if (decoratedModels.has(key)) {
            throwIfTrue(!unBoxedObject.toJSON, new MethodNotImplementedException('toJSON'));
            return this.encoder.encode([key, unBoxedObject.toJSON()]);
          }
        }
        return null;
      },
      decode: (data: Uint8Array) => {
        const array = decode(data) as [string, ReturnType<Serializable['toJSON']>];
        const [name, object] = array;
        const Model = decoratedModels.get(name) as unknown as Serializable;
        throwIfTrue(!Model, new ModelIsNotDecoratedException(name));
        throwIfTrue(!Model.fromJSON, new MethodNotImplementedException('fromJSON'));
        return Model.fromJSON(object);
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
