import {
  IdempotentCompressor,
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
  compressor?: IdempotentCompressor;
  crypto: IdempotentCrypto;
}

export class IdempotentFactory {
  public static async build({
    storage,
    logger = new ConsoleLogger(),
    serializer,
    compressor,
    crypto,
  }: IdempotentFactoryOptions) {
    if (!(await storage.isConnected())) {
      await storage.connect();
    }
    serializer.configure();
    IdempotentTransformer.configure({
      storage,
      log: logger ?? undefined,
      serializer,
      compressor,
      crypto,
    });
  }
}
