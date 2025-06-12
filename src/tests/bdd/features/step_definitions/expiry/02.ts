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
const storage = new Repository('redis://localhost:6379');
const taskInput = faker.lorem.sentence();
const taskResult = faker.lorem.sentence();
let taskUniqueId: string;
const workflowId = faker.string.uuid();
const compressor = new ZstdCompressor();

BeforeAll(async () => {
  await storage.initialize();
  transformer = IdempotentTransformer.getInstance({
    storage,
    serializer: MessagePack.getInstance(),
    compressor,
    log: new ConsoleLogger(),
  });
});

Given('no TTL is configured for state entries S2', async function () {
  const asyncTask = async (input: any) => taskResult;
  // No TTL option passed
  const wrapped = transformer.makeIdempotent(workflowId, { task: asyncTask }, { ttl: null });
  wrappedTask = wrapped.task;
});

Given('a task result "Hello, world!" is ready to be persisted S2', async function () {
  // Already set up in previous step
});

When('the task result is serialized and persisted to the state store S2', async function () {
  await wrappedTask(taskInput);
  taskUniqueId = await transformer.createHash({
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
  await storage.destroy();
});
