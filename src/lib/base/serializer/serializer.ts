import { TSerialized } from '../types/serialized.type';
import { Serializable } from './serializer.interface';

export abstract class IdempotentSerializer {
  static decoratedModels = new Map<string, new (...args: any[]) => Serializable>(); // TODO: add type
  abstract serialize<T>(data: T): Promise<TSerialized>;
  abstract deserialize<T>(data: TSerialized): Promise<T>;
  abstract configure(): void;
}
