import { decoratedModels } from './decorated-models';
import { SerializationContractViolatedException } from './errors/serialization-contract-violated.exception';
import { throwIfTrue } from './lib/throw-if-true';

export const SERIALIZE_NAME_METADATA_KEY = 'serialize_key';

/**
 * Decorator to mark a class as serializable. Users of this decorator must implement the serialize and deserialize methods.
 * @param name - The name of the class. Keep this unique.
 * @param serializeMethodName - The name of the method to serialize the class.
 * @param deserializeMethodName - The name of the method to deserialize the class. This is a static method.
 * @returns A class decorator.
 */
export function Serialize({
  name,
  serializeMethodName,
  deserializeMethodName,
}: {
  name: string;
  serializeMethodName: string;
  deserializeMethodName: string;
}): ClassDecorator {
  return (target: any) => {
    throwIfTrue(
      typeof target.prototype[serializeMethodName] !== 'function',
      new SerializationContractViolatedException(
        `Instance method "${serializeMethodName}" is not implemented. Please implement Serializable interface on your Model.`
      )
    );
    throwIfTrue(
      typeof (target as any)[deserializeMethodName] !== 'function',
      new SerializationContractViolatedException(
        `Class static method "${deserializeMethodName}" is not implemented. Please implement Deserializable interface on your Model and make sure this method is static.`
      )
    );
    Object.defineProperty(target, SERIALIZE_NAME_METADATA_KEY, {
      value: name,
      writable: false,
      configurable: false,
      enumerable: false,
    });
    decoratedModels.set(name, {
      class: target,
      serializeMethodName,
      deserializeMethodName,
    });
    return target;
  };
}
