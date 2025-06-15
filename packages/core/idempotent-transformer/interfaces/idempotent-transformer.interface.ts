import { IdempotentCompressor } from '@idempotent-transformer/base/compressor';
import { IdempotentCrypto } from '@idempotent-transformer/base/crypto';
import { IdempotentLogger } from '@idempotent-transformer/base/logger';
import { IdempotentSerializer } from '@idempotent-transformer/base/serializer';
import { IdempotentStateStore } from '@idempotent-transformer/base/state-store';

export interface IdempotentTransformerInput {
  /**
   * The logger to use for the idempotent transformer
   * @default console
   */
  log?: IdempotentLogger;
  /**
   * The state store to use for the idempotent transformer
   * @default Redis
   */
  storage: IdempotentStateStore;
  /**
   * The serializer to use for the idempotent transformer
   * @default MessagePack
   */
  serializer: IdempotentSerializer;
  /**
   * The compressor to use for the idempotent transformer
   * @default zstd
   */
  compressor?: IdempotentCompressor;

  /**
   * The crypto to use for the idempotent transformer
   * @default md5
   */
  crypto: IdempotentCrypto;
}
