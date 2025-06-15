import { Given, When, Then, BeforeAll, AfterAll } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentTransformer } from '../../../../../lib/idempotent-transformer';
import { IOptions } from '../../../../../lib/idempotent-transformer/interfaces/idempotent-options.interface';
import { ConsoleLogger } from '../../../../../lib/logger/console-logger';
import { RedisAdapter } from '../../../../../adapters/redis';
import { MessagePack } from '../../../../../adapters/message-pack';
import { ZstdCompressor } from '../../../../../adapters/zstd';
import { faker } from '@faker-js/faker';
import { IdempotentFactory } from '../../../../../lib/factory/idempotent-factory';

let transformer: IdempotentTransformer;
let wrappedTask: (input: any, options?: IOptions) => Promise<any>;
let storage: RedisAdapter;
const taskInput = faker.lorem.sentence();
const taskResult = faker.lorem.sentence();
let taskUniqueId: string;
const workflowId = faker.string.uuid();
const compressor = new ZstdCompressor();
const ttlMs = 2000; // 2 seconds

BeforeAll(async () => {
  storage = new RedisAdapter('redis://localhost:6379');
  IdempotentFactory.build({
    storage,
    serializer: MessagePack.getInstance(),
    compressor,
    logger: null,
  });
});

Given('a TTL of 2 seconds is configured for state entries S1', async function () {
  const asyncTask = async (input: any) => taskResult;
  const wrapped = IdempotentTransformer.getInstance().makeIdempotent(
    workflowId,
    {
      task: asyncTask,
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
