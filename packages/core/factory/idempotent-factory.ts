import { IdempotentCompressor } from '@idempotent-transformer/base/compressor';
import { IdempotentCrypto } from '@idempotent-transformer/base/crypto';
import { IdempotentLogger } from '@idempotent-transformer/base/logger';
import { IdempotentSerializer } from '@idempotent-transformer/base/serializer';
import { IdempotentStateStore } from '@idempotent-transformer/base/state-store';
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
