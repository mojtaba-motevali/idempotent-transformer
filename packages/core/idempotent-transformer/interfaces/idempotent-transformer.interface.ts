import {
  IdempotentCompressor,
  IdempotentCrypto,
  IdempotentLogger,
  IdempotentSerializer,
  IdempotentStateStore,
} from '../../base';

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
