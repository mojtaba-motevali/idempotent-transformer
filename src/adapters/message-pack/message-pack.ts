import { addExtension, Packr } from 'msgpackr';
import { Serializer } from '../../lib/base/serializer';
import { TSerialized } from '../../lib/base/types/serialized.type';
import { Serializable } from './message-pack.interface';
import { decoratedModels, SERIALIZE_NAME_METADATA_KEY } from './decorators/serialize.decorator';
import { ModelIsNotDecoratedException } from './errors/model-is-not-decorated.exception';
import { throwIfTrue } from './lib/throw-if-true';
import { MethodNotImplementedException } from './errors/method-not-implemented.exception';

let extPackr = new Packr({
  encodeUndefinedAsNil: false,
  mapAsEmptyObject: false,
  setAsEmptyObject: false,
  moreTypes: true,
});

export class MessagePack extends Serializer {
  private static instance: MessagePack;

  private constructor() {
    super();
    this.init();
  }

  private init() {
    Array.from(decoratedModels.values()).forEach((Model: any) => {
      addExtension({
        type: 1,
        Class: Model,
        write: (object: unknown) => {
          const unBoxedObject = object as unknown as Serializable;
          const key: string = Model[SERIALIZE_NAME_METADATA_KEY];
          throwIfTrue(!unBoxedObject.toJSON, new MethodNotImplementedException('toJSON'));
          return [key, unBoxedObject.toJSON()];
        },
        read: (data) => {
          const key = data[0];
          const value = data[1];
          const Model = decoratedModels.get(key) as unknown as Serializable;
          throwIfTrue(!Model, new ModelIsNotDecoratedException(key));
          throwIfTrue(!Model.fromJSON, new MethodNotImplementedException('fromJSON'));
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
    const encoded = extPackr.pack(data);
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

    const decoded = extPackr.unpack(buffer);

    return decoded as T;
  };
}
