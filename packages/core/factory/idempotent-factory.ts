import {
  IdempotentCheckSumGenerator,
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
  checksumGenerator: IdempotentCheckSumGenerator;
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
    storage,
    logger = new ConsoleLogger(),
    serializer,
    checksumGenerator,
  }: IdempotentFactoryOptions) {
    if (!(await storage.isConnected())) {
      await storage.connect();
    }
    return new IdempotentTransformer({
      storage,
      log: logger ?? undefined,
      serializer,
      checksumGenerator,
    });
  }
}
