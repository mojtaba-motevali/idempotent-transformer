import { Given, When, Then, BeforeAll, AfterAll } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentTransformer } from '../../../../../lib/idempotent-transformer';
import { IOptions } from '../../../../../lib/idempotent-transformer/interfaces/idempotent-options.interface';
import { RedisAdapter } from '../../../../../adapters/redis';
import { MessagePack } from '../../../../../adapters/message-pack';
import { ZstdCompressor } from '../../../../../adapters/zstd';
import { faker } from '@faker-js/faker';
import { IdempotentFactory } from '../../../../../lib/factory/idempotent-factory';

let wrappedTask: (input: any, options?: IOptions) => Promise<any>;
let storage: RedisAdapter;
const taskInput = faker.lorem.sentence();
const taskResult = faker.lorem.sentence();
const workflowId = faker.string.uuid();
const compressor = new ZstdCompressor();
let retrievedResult: string;

BeforeAll(async () => {
  storage = new RedisAdapter('redis://localhost:6379');
  IdempotentFactory.build({
    storage,
    serializer: MessagePack.getInstance(),
    compressor,
    logger: null,
  });
});

Given(
  'the compression was enabled and now is disabled and a compressed task result was previously stored in the state store',
  async function () {
    // Store the result without compression
    const asyncTask = async (input: any) => taskResult;
    const wrapped = IdempotentTransformer.getInstance().makeIdempotent(workflowId, {
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
