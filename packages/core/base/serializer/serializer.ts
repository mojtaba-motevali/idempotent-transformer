import { TSerialized } from '../types/serialized.type';

export type TDecoratedModel = {
  class: new (...args: any[]) => any;
  serializeMethodName: string;
  deserializeMethodName: string;
};

export interface IdempotentSerializer {
  /**
   * Serialize a data to a buffer.
   * @param data - The data to serialize.
   * @returns A buffer.
   */
  serialize<T>(data: T): Promise<TSerialized>;
  /**
   * Deserialize a buffer to a data.
   * @param data - The buffer to deserialize.
   * @returns The deserialized data.
   */
  deserialize<T>(data: TSerialized): Promise<T>;

  /**
   * Configure the serializer with a map of decorated models.
   * @param decoratedModels - A map of decorated models.
   */
  configure(decoratedModels: Map<string, TDecoratedModel>): void;
}
