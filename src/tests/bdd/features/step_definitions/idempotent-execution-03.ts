import { Given, When, Then, AfterAll, BeforeAll } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentTransformer } from '../../../../lib/idempotent-transformer';
import { IdempotencyKey } from '../../../../lib/idempotent-transformer/interfaces/idempotent-key.interface';
import { Options } from '../../../../lib/idempotent-transformer/interfaces/idempotent-options.interface';
import { ConsoleLogger } from '../../../../lib/logger/console-logger';
import { Repository } from '../../../../adapters/redis';
import { MessagePack } from '../../../../adapters/message-pack';
import { ZstdCompressor } from '../../../../adapters/zstd';
import { faker } from '@faker-js/faker';

let transformer: IdempotentTransformer;
let wrappedTask: (input: any, key: IdempotencyKey, options?: Options) => Promise<any>;
let retryCount = 0;
let workflowExecution = [0, 0, 0, 0];
let workflowTasks = {
  task1: async (input: any) => {
    ++workflowExecution[0];
    return input;
  },
  task2: async (input: any) => {
    ++workflowExecution[1];
    return input;
  },
  task3: async (input: any) => {
    if (retryCount < 1) throw new Error('Task 3 failed');
    ++workflowExecution[2];
  },
  task4: async (input: any) => {
    ++workflowExecution[3];
    return input;
  },
};

let input = faker.string.uuid();
const idempotentWorkflowKey = faker.string.uuid();
let storage: Repository;
const idempotencyKeys = [
  faker.string.uuid(),
  faker.string.uuid(),
  faker.string.uuid(),
  faker.string.uuid(),
];
let taskExecutionCount = 0;
let currentDate = new Date();

BeforeAll(async () => {
  storage = new Repository('redis://localhost:6379');
  await storage.initialize();

  transformer = IdempotentTransformer.getInstance({
    storage,
    serializer: MessagePack.getInstance(),
    compressor: new ZstdCompressor(),
    log: new ConsoleLogger(),
  });
});

// Given a workflow including 4 tasks
// When I execute 2 tasks successfully and third one fails
// Then I retry execution of the all tasks
// Then the library does not execute the first two tasks and successfully executes all tasks.

// Setup transformer with in-memory storage for testing
Given('a workflow including 4 tasks', async function () {});
When('I execute 2 tasks successfully and third one fails', async function () {
  const wrapped = transformer.makeIdempotent(idempotentWorkflowKey, { ...workflowTasks });
  let error: unknown;
  try {
    const result1 = await wrapped.task1(input, { key: idempotencyKeys[0] });
    const result2 = await wrapped.task2(result1, { key: idempotencyKeys[1] });
    await wrapped.task3(result2, { key: idempotencyKeys[2] });
  } catch (err) {
    error = err;
  }
  expect(error).to.be.instanceOf(Error);
});

Then('I retry execution of the all tasks', async function () {
  ++retryCount;
  const wrapped = transformer.makeIdempotent(idempotentWorkflowKey, { ...workflowTasks });
  await wrapped.task1(input, { key: idempotencyKeys[0] });
  await wrapped.task2(input, { key: idempotencyKeys[1] });
  await wrapped.task3(input, { key: idempotencyKeys[2] });
  await wrapped.task4(input, { key: idempotencyKeys[3] });
});

Then(
  'the task should execute successfully and all tasks should have been executed only once',
  async function () {
    expect(workflowExecution[0]).to.equal(1);
    expect(workflowExecution[1]).to.equal(1);
    expect(workflowExecution[2]).to.equal(1);
    expect(workflowExecution[3]).to.equal(1);
  }
);

AfterAll(async () => {
  await storage.destroy();
});
