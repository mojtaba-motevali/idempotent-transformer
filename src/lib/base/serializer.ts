import { TSerializable } from './types/serializable.type';

export abstract class Serializer {
  abstract serialize: <T>(data: T) => Promise<TSerializable>;
  abstract deserialize: <T>(data: TSerializable) => Promise<T>;
}
