import { TSerialized } from '../types/serialized.type';

export abstract class IdempotentSerializer {
  /**
   * A map of decorated models.
   */
  static decoratedModels = new Map<
    string,
    {
      class: ClassDecorator;
      /**
       * The name of the method to serialize the class.
       */
      serializeMethodName: string;
      /**
       * The name of the method to deserialize the class. This is a static method.
       */
      deserializeMethodName: string;
    }
  >();
  abstract serialize<T>(data: T): Promise<TSerialized>;
  abstract deserialize<T>(data: TSerialized): Promise<T>;
  abstract configure(): void;
}
