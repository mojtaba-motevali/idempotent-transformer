import { Serializable } from '../message-pack.interface';

export const decoratedModels = new Map<string, new (...args: any[]) => Serializable>();

export const SERIALIZE_NAME_METADATA_KEY = 'serialize_key';

/**
 * Decorator to mark a class as serializable. Users of this decorator must implement the Serializable interface.
 * @interface Serializable - The interface that must be implemented by the class.
 * @param name - The name of the class.
 * @returns A class decorator.
 */
export function Serialize({ name }: { name: string }): ClassDecorator {
  return (target: any) => {
    decoratedModels.set(name, target);
    Object.defineProperty(target, SERIALIZE_NAME_METADATA_KEY, {
      value: name,
      writable: false,
      configurable: false,
      enumerable: false,
    });
    return target;
  };
}
