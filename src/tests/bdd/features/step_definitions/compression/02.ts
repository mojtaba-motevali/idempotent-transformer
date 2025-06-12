import { Given, When, Then, BeforeAll, AfterAll } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentTransformer } from '../../../../../lib/idempotent-transformer';
import { IOptions } from '../../../../../lib/idempotent-transformer/interfaces/idempotent-options.interface';
import { ConsoleLogger } from '../../../../../lib/logger/console-logger';
import { Repository } from '../../../../../adapters/redis';
import { MessagePack } from '../../../../../adapters/message-pack';
import { ZstdCompressor } from '../../../../../adapters/zstd';
import { faker } from '@faker-js/faker';

let transformer: IdempotentTransformer;
let wrappedTask: (input: any, options?: IOptions) => Promise<any>;
let storage: Repository;
const taskInput = faker.lorem.sentence();
const taskResult = faker.lorem.sentence();
let taskUniqueId: string;
const workflowId = faker.string.uuid();
const compressor = new ZstdCompressor();
let retrievedResult: string;

BeforeAll(async () => {
  storage = new Repository('redis://localhost:6379');
  await storage.initialize();
  transformer = IdempotentTransformer.getInstance({
    storage,
    serializer: MessagePack.getInstance(),
    compressor,
    log: new ConsoleLogger(),
  });
});

Given('compression is enabled in the library configuration', async function () {
  // Already set in BeforeAll
});

Given('a compressed task result is stored in the state store', async function () {
  const asyncTask = async (input: any) => taskResult;
  const wrapped = transformer.makeIdempotent(workflowId, { task: asyncTask });
  wrappedTask = wrapped.task;
  // Persist the result with compression enabled
  await wrappedTask(taskInput, { shouldCompress: true });
  // Compute the taskUniqueId for direct state store access
  taskUniqueId = await transformer.createHash({
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
  await storage.destroy();
});
