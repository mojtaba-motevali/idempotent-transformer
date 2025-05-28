import { Compressor } from '../../base/compressor';
import { Logger } from '../../base/logger';
import { Serializer } from '../../base/serializer';
import { StateStore } from '../../base/state-store';

export interface IdempotentTransformerInput {
  /**
   * The logger to use for the idempotent transformer
   * @default console
   */
  log: Logger;
  /**
   * The state store to use for the idempotent transformer
   * @default Redis
   */
  storage: StateStore;
  /**
   * The serializer to use for the idempotent transformer
   * @default MessagePack
   */
  serializer: Serializer;
  /**
   * The compressor to use for the idempotent transformer
   * @default zstd
   */
  compressor: Compressor;
}

export interface IdempotentTransformerOutput {}
