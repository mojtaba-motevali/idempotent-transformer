import { Given, When, Then, AfterAll, BeforeAll } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentTransformer } from '../../../../../lib/idempotent-transformer';
import { RedisAdapter } from '../../../../../adapters/redis';
import { MessagePack } from '../../../../../adapters/message-pack';
import { ZstdCompressor } from '../../../../../adapters/zstd';
import { faker } from '@faker-js/faker';
import { IdempotentFactory } from '../../../../../lib/factory/idempotent-factory';

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

const input = faker.string.uuid();
const idempotentWorkflowKey = faker.string.uuid();
const storage: RedisAdapter = new RedisAdapter('redis://localhost:6379');

BeforeAll(async () => {
  await IdempotentFactory.build({
    storage,
    serializer: MessagePack.getInstance(),
    compressor: new ZstdCompressor(),
    logger: null,
  });
});

// Given a workflow including 4 tasks
// When I execute 2 tasks successfully and third one fails
// Then I retry execution of the all tasks
// Then the library does not execute the first two tasks and successfully executes all tasks.

// Setup transformer with in-memory storage for testing
Given('a workflow including 4 tasks', async function () {});
When('I execute 2 tasks successfully and third one fails', async function () {
  const wrapped = IdempotentTransformer.getInstance().makeIdempotent(idempotentWorkflowKey, {
    ...workflowTasks,
  });
  let error: unknown;
  try {
    const result1 = await wrapped.task1(input);
    const result2 = await wrapped.task2(result1);
    await wrapped.task3(result2);
  } catch (err) {
    error = err;
  }
  expect(error).to.be.instanceOf(Error);
});

Then('I retry execution of the all tasks', async function () {
  ++retryCount;
  const wrapped = IdempotentTransformer.getInstance().makeIdempotent(idempotentWorkflowKey, {
    ...workflowTasks,
  });
  await wrapped.task1(input);
  await wrapped.task2(input);
  await wrapped.task3(input);
  await wrapped.task4(input);
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
  await storage.disconnect();
});
