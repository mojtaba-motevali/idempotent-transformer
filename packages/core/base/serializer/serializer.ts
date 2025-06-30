import { TSerialized } from '../types/serialized.type';

export type TDecoratedModel = {
  class: new (...args: any[]) => any;
  serializeMethodName: string;
  deserializeMethodName: string;
};

export abstract class IdempotentSerializer {
  /**
   * A map of decorated models.
   */
  static decoratedModels = new Map<string, TDecoratedModel>();
  abstract serialize<T>(data: T): Promise<TSerialized>;
  abstract deserialize<T>(data: TSerialized): Promise<T>;
  abstract configure(models: Map<string, TDecoratedModel>): void;
}
