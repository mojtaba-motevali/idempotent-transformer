import { AfterAll, BeforeAll, Given, Then, When } from '@cucumber/cucumber';
import { expect } from 'chai';
import { RedisAdapter } from '@idempotent-transformer/adapter-redis';
import { MessagePack } from '@idempotent-transformer/adapter-message-pack';
import { ZstdCompressor } from '@idempotent-transformer/adapter-zstd';
import { faker } from '@faker-js/faker';
import {
  IdempotentFactory,
  IdempotentTransformer,
  IdempotencyConflictException,
} from '@idempotent-transformer/core';
import { Md5Adapter } from '@idempotent-transformer/adapter-crypto';

let asyncTask: (input: any) => Promise<any>;
const workflowId = faker.string.uuid();
const input = faker.string.uuid();
const storage = new RedisAdapter({
  options: {
    host: 'localhost',
    port: 6379,
  },
});

BeforeAll(async () => {
  await IdempotentFactory.build({
    storage,
    serializer: MessagePack.getInstance(),
    compressor: new ZstdCompressor(),
    logger: null,
    crypto: new Md5Adapter(),
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
    const wrapped = await IdempotentTransformer.getInstance().makeIdempotent(workflowId, {
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
    const wrapped = await IdempotentTransformer.getInstance().makeIdempotent(workflowId, {
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
