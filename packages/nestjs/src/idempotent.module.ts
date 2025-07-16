import { DynamicModule, Module, Provider } from '@nestjs/common';
import { IdempotentFactory, IdempotentFactoryOptions } from '@idempotent-transformer/core';

export const IDEMPOTENT_MODULE_OPTIONS = 'IDEMPOTENT_MODULE_OPTIONS';

export const IDEMPOTENT_TRANSFORMER = 'IDEMPOTENT_TRANSFORMER';

interface IdempotentTransformerRegisterAsyncOptions {
  useFactory: (...args: any[]) => Promise<IdempotentFactoryOptions> | IdempotentFactoryOptions;
  inject?: any[];
  imports?: any[];
}

@Module({})
export class IdempotentModule {
  static async registerAsync(
    opts: IdempotentTransformerRegisterAsyncOptions
  ): Promise<DynamicModule> {
    const optionsProvider: Provider = {
      provide: IDEMPOTENT_MODULE_OPTIONS,
      useFactory: opts.useFactory,
      inject: opts.inject || [],
    };

    const transformerProvider: Provider = {
      provide: IDEMPOTENT_TRANSFORMER,
      useFactory: async (options: IdempotentFactoryOptions) => {
        const factory = IdempotentFactory.getInstance();
        const idempotencyTransformer = await factory.build(options);
        return idempotencyTransformer;
      },
      inject: [IDEMPOTENT_MODULE_OPTIONS],
    };

    return {
      module: IdempotentModule,
      imports: opts.imports || [],
      providers: [optionsProvider, transformerProvider],
      exports: [transformerProvider],
    };
  }
}
