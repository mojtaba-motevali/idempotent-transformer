import { Options } from '../idempotent-transformer/interfaces/idempotent-options.interface';
import { TSerializable } from './types/serializable.type';

interface IContext {
  methodName: string;
  options?: Options;
}

export abstract class StateStore {
  abstract find: (id: string) => Promise<TSerializable | null>;
  abstract save: (id: string, value: TSerializable, context: IContext) => Promise<void>;
}
