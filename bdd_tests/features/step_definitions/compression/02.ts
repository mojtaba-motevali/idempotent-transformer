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
let retrievedResult: string;

BeforeAll(async () => {
  storage = new RedisAdapter({
    option: {
      url: 'redis://localhost:6379',
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

Given('compression is enabled in the library configuration', async function () {
  // Already set in BeforeAll
});

Given('a compressed task result is stored in the state store', async function () {
  const wrapped = await IdempotentTransformer.getInstance().makeIdempotent(workflowId, {
    task: async (input: any) => taskResult,
  });
  wrappedTask = wrapped.task;
  // Persist the result with compression enabled
  await wrappedTask(taskInput, { shouldCompress: true });
  // Compute the taskUniqueId for direct state store access
  taskUniqueId = await IdempotentTransformer.getInstance().createHash({
    workflowId,
  });
});

When(
  'the task result is retrieved from the state store and decompress the data transparently',
  async function () {
    // This should transparently decompress and deserialize
    retrievedResult = await wrappedTask(taskInput, { shouldCompress: true });
  }
);

Then("retrieved result should match the original task's result.", async function () {
  expect(retrievedResult).to.equal(taskResult);
});

AfterAll(async () => {
  await storage.disconnect();
});
