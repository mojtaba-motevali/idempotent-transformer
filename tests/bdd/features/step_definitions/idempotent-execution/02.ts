import { AfterAll, BeforeAll, Given, Then, When } from '@cucumber/cucumber';
import { expect } from 'chai';
import { RedisAdapter } from '../../../../../packages/adapter-redis/redis';
import { MessagePack } from '../../../../../packages/adapter-message-pack/message-pack';
import { ZstdCompressor } from '../../../../../packages/adapter-zstd/zstd-compressor';
import { faker } from '@faker-js/faker';
import { IdempotencyConflictException } from '../../../../../packages/core/idempotent-transformer/exceptions/conflict.exception';
import { IdempotentFactory } from '../../../../../packages/core/factory/idempotent-factory';
import { IdempotentTransformer } from '../../../../../packages/core/idempotent-transformer';

let asyncTask: (input: any) => Promise<any>;
const workflowId = faker.string.uuid();
const input = faker.string.uuid();
const storage = new RedisAdapter('redis://localhost:6379');

BeforeAll(async () => {
  await IdempotentFactory.build({
    storage,
    serializer: MessagePack.getInstance(),
    compressor: new ZstdCompressor(),
    logger: null,
  });
});

Given('an asynchronous task that accepts parameters', async function () {
  asyncTask = async (input: any) => {
    return {
      a: input,
    };
  };
});

When(
  'I wrap the task with the idempotent execution wrapper and I execute the wrapped task with parameter "A" successfully',
  async () => {
    const wrapped = IdempotentTransformer.getInstance().makeIdempotent(workflowId, {
      task: asyncTask,
    });
    await wrapped.task({
      input,
    });
  }
);

Then(
  'the task fails with Conflict exception when I execute the wrapped task with parameter "B"',
  async () => {
    const wrapped = IdempotentTransformer.getInstance().makeIdempotent(workflowId, {
      task: asyncTask,
    });
    let error: unknown;
    try {
      await wrapped.task({
        input: faker.string.uuid(),
      });
    } catch (err) {
      error = err;
    }
    expect(error).to.be.instanceOf(IdempotencyConflictException);
  }
);

AfterAll(async () => {
  await storage.disconnect();
});
