import { Logger, Module } from '@nestjs/common';
import { AppController } from './app.controller';
import { AppService } from './app.service';
import { IdempotentModule } from '@idempotent-transformer/nestjs';
import { ZstdCompressor } from '@idempotent-transformer/adapter-zstd';
import { RedisAdapter } from '@idempotent-transformer/adapter-redis';
import { MessagePack } from '@idempotent-transformer/adapter-message-pack';
import { Md5Adapter } from '@idempotent-transformer/adapter-crypto';
import { ExampleService } from './example/example.service';

@Module({
  imports: [
    IdempotentModule.registerAsync({
      useFactory: async () => {
        const storage = new RedisAdapter('redis://localhost:6379');
        await storage.connect();
        return {
          adapter: new ZstdCompressor(),
          storage,
          serializer: MessagePack.getInstance(),
          crypto: new Md5Adapter(),
          logger: new Logger(),
        };
      },
    }),
  ],
  controllers: [AppController],
  providers: [AppService, ExampleService],
})
export class AppModule {}
