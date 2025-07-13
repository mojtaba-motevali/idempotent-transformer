import { TSerialized } from '../types/serialized.type';

export interface IdempotentCheckSumGenerator {
  generate<T extends TSerialized>(value: T): Promise<string | number>;
}
