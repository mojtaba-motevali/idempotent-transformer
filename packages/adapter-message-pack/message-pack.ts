import { addExtension, Packr } from 'msgpackr';
import {
  IdempotentSerializer,
  Serializable,
  SERIALIZE_NAME_METADATA_KEY,
  TSerialized,
} from '@idempotent-transformer/base';
import { ModelIsNotDecoratedException } from './errors';

export class MessagePack extends IdempotentSerializer {
  private static instance: MessagePack;
  private extPackr: Packr;

  private constructor() {
    super();
    this.extPackr = new Packr({
      encodeUndefinedAsNil: false,
      mapAsEmptyObject: false,
      setAsEmptyObject: false,
      moreTypes: true,
    });
  }

  configure() {
    Array.from(IdempotentSerializer.decoratedModels.values()).forEach((Model: any) => {
      addExtension({
        type: 1,
        Class: Model,
        write: (object: unknown) => {
          const unBoxedObject = object as unknown as Serializable;
          const key: string = Model[SERIALIZE_NAME_METADATA_KEY];
          return [key, unBoxedObject.toJSON()];
        },
        read: (data) => {
          const key = data[0];
          const value = data[1];
          const Model = IdempotentSerializer.decoratedModels.get(key) as unknown as Serializable;
          if (!Model) {
            throw new ModelIsNotDecoratedException(key);
          }
          return Model.fromJSON(value);
        },
      });
    });
  }

  static getInstance() {
    if (!MessagePack.instance) {
      MessagePack.instance = new MessagePack();
    }
    return MessagePack.instance;
  }

  serialize = async <T>(data: T): Promise<TSerialized> => {
    const encoded = this.extPackr.pack(data);
    return encoded;
  };

  deserialize = async <T>(data: TSerialized): Promise<T> => {
    let buffer: Uint8Array;

    if (data instanceof Blob) {
      buffer = new Uint8Array(await data.arrayBuffer());
    } else if (data instanceof Uint8Array) {
      buffer = data;
    } else {
      throw new Error('Input data must be either a Blob or Uint8Array');
    }

    const decoded = this.extPackr.unpack(buffer);

    return decoded as T;
  };
}
