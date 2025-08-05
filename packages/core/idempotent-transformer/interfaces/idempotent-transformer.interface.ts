import { IdempotentLogger, IdempotentSerializer } from '../../base';
import { IdempotentRpcAdapter } from '../../base/rpc-adapter';

export interface IdempotentTransformerInput {
  /**
   * The logger to use for the idempotent transformer
   * @default console
   */
  log?: IdempotentLogger;

  /**
   * The serializer to use for the idempotent transformer
   * @default MessagePack
   */
  serializer: IdempotentSerializer;

  /**
   * The rpc adapter to use for the idempotent transformer
   */
  rpcAdapter: IdempotentRpcAdapter;
}
