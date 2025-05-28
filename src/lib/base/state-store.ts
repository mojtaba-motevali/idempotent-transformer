import { Options } from '../idempotent-transformer/interfaces/idempotent-options.interface';
import { TBinary } from './types/binary.type';

interface IContext {
  methodName: string;
  options?: Options;
}

export abstract class StateStore {
  abstract find: (id: string) => Promise<TBinary | null>;
  abstract save: (id: string, value: TBinary, context: IContext) => Promise<void>;
}
