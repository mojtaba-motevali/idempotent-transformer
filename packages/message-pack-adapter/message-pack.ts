import { addExtension, Packr } from 'msgpackr';
import {
  IdempotentSerializer,
  SERIALIZE_NAME_METADATA_KEY,
  TDecoratedModel,
  TSerialized,
} from '@idempotent-transformer/core';
import { ModelIsNotDecoratedException } from './errors';

export class MessagePack implements IdempotentSerializer {
  private static instance: MessagePack;
  private extPackr: Packr;

  private constructor() {
    this.extPackr = new Packr({
      encodeUndefinedAsNil: false,
      mapAsEmptyObject: false,
      setAsEmptyObject: false,
      moreTypes: true,
    });
  }

  configure(models: Map<string, TDecoratedModel>) {
    Array.from(models.values()).forEach(({ class: Model, serializeMethodName }) => {
      addExtension({
        type: 1,
        Class: Model,
        write: (object: unknown) => {
          const unBoxedObject = object as any;
          const key: string = (Model as any)[SERIALIZE_NAME_METADATA_KEY];
          return [key, unBoxedObject[serializeMethodName]()];
        },
        read: (data) => {
          const key = data[0];
          const value = data[1];
          const Model = models.get(key);
          if (!Model) {
            throw new ModelIsNotDecoratedException(key);
          }
          const { deserializeMethodName, class: ModelClass } = Model;
          return (ModelClass as any)[deserializeMethodName](value);
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
