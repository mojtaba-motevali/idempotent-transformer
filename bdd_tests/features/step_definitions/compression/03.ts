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

Given('compression is now enabled in the library configuration', async function () {
  // Already set in BeforeAll
});

Given('an uncompressed task result was previously stored in the state store', async function () {
  // Store the result without compression
  const wrapped = await IdempotentTransformer.getInstance().makeIdempotent(workflowId, {
    task: async (input: any) => taskResult,
  });
  wrappedTask = wrapped.task;
  await wrappedTask(taskInput, { shouldCompress: false });
});

When(
  'the task result is retrieved from the state store and should correctly retrieve the uncompressed result',
  async function () {
    // Now compression is enabled, but the stored data is uncompressed
    retrievedResult = await wrappedTask(taskInput, { shouldCompress: true });
  }
);

Then("retrieved result should match the original task's result", async function () {
  expect(retrievedResult).to.equal(taskResult);
});

AfterAll(async () => {
  await storage.disconnect();
});
