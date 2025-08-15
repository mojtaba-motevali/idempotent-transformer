import { Logger, Module } from '@nestjs/common';
import { AppController } from './app.controller';
import { AppService } from './app.service';
import { ExampleService } from './example/example.service';
import { IdempotentModule } from '@idempotent-transformer/nestjs';
import { MessagePack } from '@idempotent-transformer/message-pack-adapter';
import { GrpcAdapter } from '@idempotent-transformer/grpc-adapter';

@Module({
  imports: [
    IdempotentModule.registerAsync({
      useFactory: () => ({
        serializer: MessagePack.getInstance(),
        logger: new Logger(),
        rpcAdapter: new GrpcAdapter({
          host: 'localhost',
          port: 51000,
        }),
      }),
    }),
  ],
  controllers: [AppController],
  providers: [AppService, ExampleService],
})
export class AppModule {}
