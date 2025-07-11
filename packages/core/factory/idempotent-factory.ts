import {
  IdempotentCrypto,
  IdempotentLogger,
  IdempotentSerializer,
  IdempotentStateStore,
} from '../base';
import { IdempotentTransformer } from '../idempotent-transformer';
import { ConsoleLogger } from '../logger/console-logger';

export interface IdempotentFactoryOptions {
  storage: IdempotentStateStore;
  logger?: IdempotentLogger | null;
  serializer: IdempotentSerializer;
  crypto: IdempotentCrypto;
}

export class IdempotentFactory {
  public static async build({
    storage,
    logger = new ConsoleLogger(),
    serializer,
    crypto,
  }: IdempotentFactoryOptions) {
    if (!(await storage.isConnected())) {
      await storage.connect();
    }
    return new IdempotentTransformer({
      storage,
      log: logger ?? undefined,
      serializer,
      crypto,
    });
  }
}
