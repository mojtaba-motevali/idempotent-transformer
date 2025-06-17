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
  storage = new RedisAdapter('redis://localhost:6379');
  await IdempotentFactory.build({
    storage,
    serializer: MessagePack.getInstance(),
    compressor,
    logger: null,
    crypto: new Md5Adapter(),
  });
});

Given(
  'the compression was enabled and now is disabled and a compressed task result was previously stored in the state store',
  async function () {
    // Store the result without compression
    const asyncTask = async (input: any) => taskResult;
    const wrapped = await IdempotentTransformer.getInstance().makeIdempotent(workflowId, {
      task: asyncTask,
    });
    wrappedTask = wrapped.task;
    await wrappedTask(taskInput, { shouldCompress: true });
    await IdempotentTransformer.getInstance().createHash({
      workflowId,
    });
  }
);

When(
  'the task result is retrieved with compression disabled from the state store, it should correclty be decompressed',
  async function () {
    // Now compression is enabled, but the stored data is uncompressed
    retrievedResult = await wrappedTask(taskInput, { shouldCompress: false });
  }
);

Then("retrieved result must match the original task's result.", async function () {
  expect(retrievedResult).to.equal(taskResult);
});

AfterAll(async () => {
  await storage.disconnect();
});
