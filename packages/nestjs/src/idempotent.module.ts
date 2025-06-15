import { DynamicModule, Module, Provider } from '@nestjs/common';
import {
  IdempotentFactory,
  IdempotentFactoryOptions,
  IdempotentTransformer,
} from '@idempotent-transformer/core';

export const IDEMPOTENT_TRANSFORMER = 'IDEMPOTENT_TRANSFORMER';

@Module({})
export class IdempotentModule {
  static async registerAsync(options: IdempotentFactoryOptions): Promise<DynamicModule> {
    await IdempotentFactory.build(options);

    const idempotentProvider: Provider = {
      provide: IDEMPOTENT_TRANSFORMER,
      useValue: IdempotentTransformer.getInstance(), // Already configured statically by the factory
    };

    return {
      module: IdempotentModule,
      providers: [idempotentProvider],
      exports: [idempotentProvider],
    };
  }
}
