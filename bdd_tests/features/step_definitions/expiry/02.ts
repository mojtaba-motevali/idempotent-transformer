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
const storage = new RedisAdapter('redis://localhost:6379');
const taskInput = faker.lorem.sentence();
const taskResult = faker.lorem.sentence();
let taskUniqueId: string;
const workflowId = faker.string.uuid();
const compressor = new ZstdCompressor();

BeforeAll(async () => {
  await IdempotentFactory.build({
    storage,
    serializer: MessagePack.getInstance(),
    compressor,
    logger: null,
    crypto: new Md5Adapter(),
  });
});

Given('no TTL is configured for state entries S2', async function () {
  // No TTL option passed
  const wrapped = await IdempotentTransformer.getInstance().makeIdempotent(
    workflowId,
    {
      task: async (input: any) => taskResult,
    },
    { ttl: null }
  );
  wrappedTask = wrapped.task;
});

Given('a task result "Hello, world!" is ready to be persisted S2', async function () {
  // Already set up in previous step
});

When('the task result is serialized and persisted to the state store S2', async function () {
  await wrappedTask(taskInput);
  taskUniqueId = await IdempotentTransformer.getInstance().createHash({
    workflowId,
    contextName: 'task',
  });
});

Then('the persisted entry should exist in the state store S2', async function () {
  const persisted = await storage.find(taskUniqueId);
  expect(persisted).to.not.be.null;
});

Then('the entry should not expire automatically S2', async function () {
  await new Promise((resolve) => setTimeout(resolve, 3000));
  const persisted = await storage.find(taskUniqueId);
  expect(persisted).to.not.be.null;
});

AfterAll(async () => {
  await storage.disconnect();
});
