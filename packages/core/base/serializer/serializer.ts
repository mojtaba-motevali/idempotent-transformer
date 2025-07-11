import { TSerialized } from '../types/serialized.type';

export type TDecoratedModel = {
  class: new (...args: any[]) => any;
  serializeMethodName: string;
  deserializeMethodName: string;
};

export interface IdempotentSerializer {
  /**
   * A map of decorated models.
   */
  serialize<T>(data: T): Promise<TSerialized>;
  deserialize<T>(data: TSerialized): Promise<T>;
  /**
   * Configure the serializer with a map of decorated models.
   * @param decoratedModels - A map of decorated models.
   */
  configure(decoratedModels: Map<string, TDecoratedModel>): void;
}
