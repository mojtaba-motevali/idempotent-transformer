import { throwIfTrue } from './lib/throw-if-true';
import { IdempotentSerializer } from './serializer';
import { Serializable } from './serializer.interface';

export const SERIALIZE_NAME_METADATA_KEY = 'serialize_key';
const serializeMethodName: keyof Serializable = 'toJSON';
const deserializeMethodName: keyof Serializable = 'fromJSON';

/**
 * Decorator to mark a class as serializable. Users of this decorator must implement the Serializable interface.
 * @interface Serializable - The interface that must be implemented by the class.
 * @param name - The name of the class. Keep this unique.
 * @returns A class decorator.
 */
export function Serialize({ name }: { name: string }): ClassDecorator {
  return (target: any) => {
    throwIfTrue(
      typeof target.prototype[serializeMethodName] !== 'function',
      new Error(
        `Instance method "${serializeMethodName}" is not implemented. Please implement Serializable interface on your Model.`
      )
    );
    throwIfTrue(
      typeof (target as any)[deserializeMethodName] !== 'function',
      new Error(
        `Class static method "${deserializeMethodName}" is not implemented. Please implement Serializable interface on your Model and make sure this method is static.`
      )
    );
    Object.defineProperty(target, SERIALIZE_NAME_METADATA_KEY, {
      value: name,
      writable: false,
      configurable: false,
      enumerable: false,
    });
    IdempotentSerializer.decoratedModels.set(name, target);
    return target;
  };
}
