import { Given, When, Then, BeforeAll, AfterAll } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentTransformer, IIdempotentTaskOptions } from '@idempotent-transformer/core';
import { RedisAdapter } from '@idempotent-transformer/adapter-redis';
import { MessagePack } from '@idempotent-transformer/adapter-message-pack';
import { ZstdCompressor } from '@idempotent-transformer/adapter-zstd';
import { faker } from '@faker-js/faker';
import { IdempotentFactory } from '@idempotent-transformer/core';
import { Md5Adapter } from '@idempotent-transformer/adapter-crypto';

let wrappedTask: (input: any, options?: IIdempotentTaskOptions) => Promise<any>;
let storage: RedisAdapter;
const taskInput = faker.lorem.sentence();
const taskResult = faker.lorem.sentence();
let taskUniqueId: string;
const workflowId = faker.string.uuid();
const compressor = new ZstdCompressor();
const ttlMs = 2000; // 2 seconds

BeforeAll(async () => {
  storage = new RedisAdapter({
    options: {
      host: 'localhost',
      port: 6379,
    },
  });
  await IdempotentFactory.build({
    storage,
    serializer: MessagePack.getInstance(),
    compressor,
    logger: null,
    crypto: new Md5Adapter(),
  });
});

Given('a TTL of 2 seconds is configured for state entries S1', async function () {
  const wrapped = await IdempotentTransformer.getInstance().makeIdempotent(
    workflowId,
    {
      task: async (input: any) => taskResult,
    },
    { ttl: ttlMs }
  );
  wrappedTask = wrapped.task;
});

Given('a task result "Hello, world!" is ready to be persisted S1', async function () {
  // Already set up in previous step
});

When('the task result is serialized and persisted to the state store S1', async function () {
  await wrappedTask(taskInput);
  taskUniqueId = await IdempotentTransformer.getInstance().createHash({
    workflowId,
    contextName: 'task',
  });
});

Then('the persisted entry should exist in the state store immediately S1', async function () {
  const persisted = await storage.find(taskUniqueId);
  expect(persisted).to.not.be.null;
});

Then('after 3 seconds, the entry should be expired and no longer available S1', async function () {
  await new Promise((resolve) => setTimeout(resolve, 3000));
  const persisted = await storage.find(taskUniqueId);
  expect(persisted).to.be.null;
});

AfterAll(async () => {
  await storage.disconnect();
});
