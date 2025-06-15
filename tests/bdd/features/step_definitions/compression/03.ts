import { Given, When, Then, BeforeAll, AfterAll } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentTransformer } from '../../../../../packages/core/idempotent-transformer';
import { IOptions } from '../../../../../packages/core/idempotent-transformer/interfaces/idempotent-options.interface';
import { RedisAdapter } from '../../../../../packages/adapter-redis/redis';
import { MessagePack } from '../../../../../packages/adapter-message-pack/message-pack';
import { ZstdCompressor } from '../../../../../packages/adapter-zstd/zstd-compressor';
import { faker } from '@faker-js/faker';
import { IdempotentFactory } from '../../../../../packages/core/factory/idempotent-factory';

let wrappedTask: (input: any, options?: IOptions) => Promise<any>;
let storage: RedisAdapter;
const taskInput = faker.lorem.sentence();
const taskResult = faker.lorem.sentence();
const workflowId = faker.string.uuid();
const compressor = new ZstdCompressor();
let retrievedResult: string;

BeforeAll(async () => {
  storage = new RedisAdapter('redis://localhost:6379');
  await IdempotentFactory.build({
    storage,
    serializer: MessagePack.getInstance(),
    compressor,
    logger: null,
  });
});

Given('compression is now enabled in the library configuration', async function () {
  // Already set in BeforeAll
});

Given('an uncompressed task result was previously stored in the state store', async function () {
  // Store the result without compression
  const wrapped = IdempotentTransformer.getInstance().makeIdempotent(workflowId, {
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
