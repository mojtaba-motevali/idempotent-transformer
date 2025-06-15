import { Md5Adapter } from '../../adapters/crypto/crypto';
import { MessagePack } from '../../adapters/message-pack';
import { IdempotentCompressor } from '../base/compressor';
import { IdempotentCrypto } from '../base/crypto';
import { IdempotentLogger } from '../base/logger';
import { IdempotentSerializer } from '../base/serializer';
import { IdempotentStateStore } from '../base/state-store';
import { IdempotentTransformer } from '../idempotent-transformer';
import { ConsoleLogger } from '../logger/console-logger';

export interface IdempotentFactoryOptions {
  storage: IdempotentStateStore;
  logger?: IdempotentLogger | null;
  serializer: IdempotentSerializer;
  compressor?: IdempotentCompressor;
  crypto?: IdempotentCrypto;
}

export class IdempotentFactory {
  public static async build({
    storage,
    logger = new ConsoleLogger(),
    serializer = MessagePack.getInstance(),
    compressor,
    crypto = new Md5Adapter(),
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
