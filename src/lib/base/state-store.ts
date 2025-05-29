import { Options } from '../idempotent-transformer/interfaces/idempotent-options.interface';

interface IContext {
  methodName: string;
  options?: Options;
}

export abstract class StateStore {
  abstract find(id: string): Promise<Uint8Array<ArrayBufferLike> | null>;
  abstract save(id: string, value: Uint8Array<ArrayBufferLike>, context: IContext): Promise<void>;
}
