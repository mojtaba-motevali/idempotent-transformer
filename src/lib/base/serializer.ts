import { TSerialized } from './types/serialized.type';

export abstract class Serializer {
  abstract serialize<T>(data: T): Promise<TSerialized>;
  abstract deserialize<T>(data: TSerialized): Promise<T>;
}
