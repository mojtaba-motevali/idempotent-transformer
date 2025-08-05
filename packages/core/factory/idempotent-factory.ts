import { IdempotentLogger, IdempotentSerializer, IdempotentRpcAdapter } from '../base';
import { IdempotentTransformer } from '../idempotent-transformer';
import { ConsoleLogger } from '../logger/console-logger';

export interface IdempotentFactoryOptions {
  rpcAdapter: IdempotentRpcAdapter;
  logger?: IdempotentLogger | null;
  serializer: IdempotentSerializer;
}

export class IdempotentFactory {
  private static instance: IdempotentFactory;

  private constructor() {}

  public static getInstance() {
    if (!this.instance) {
      this.instance = new IdempotentFactory();
    }
    return this.instance;
  }

  public async build({
    rpcAdapter,
    logger = new ConsoleLogger(),
    serializer,
  }: IdempotentFactoryOptions) {
    return new IdempotentTransformer({
      rpcAdapter,
      log: logger ?? undefined,
      serializer,
    });
  }
}
