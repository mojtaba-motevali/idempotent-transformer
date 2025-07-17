import { Given, When, Then, AfterAll, BeforeAll } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentRunnerResult, IdempotentTransformer } from '@idempotent-transformer/core';
import { faker } from '@faker-js/faker';
import { IdempotentFactory } from '@idempotent-transformer/core';
import { CheckSumGenerator } from '@idempotent-transformer/checksum-adapter';
import { MessagePack } from '@idempotent-transformer/message-pack-adapter';
import { GrpcAdapter } from '@idempotent-transformer/grpc-adapter';

let retryCount = 0;
let workflowExecution = [0, 0, 0, 0];
let workflowTasks = {
  task1: async (input: any) => {
    ++workflowExecution[0];
    return input;
  },
  task2: async (input: any) => {
    workflowExecution[1] += 1;
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

let transformer: IdempotentTransformer;

const input = faker.string.uuid();
const idempotentWorkflowKey = faker.string.uuid();
const rpcAdapter: GrpcAdapter = new GrpcAdapter({
  host: 'localhost',
  port: 51000,
});
let runner: IdempotentRunnerResult;
BeforeAll(async () => {
  transformer = await IdempotentFactory.getInstance().build({
    rpcAdapter,
    serializer: MessagePack.getInstance(),
    logger: console,
    checksumGenerator: new CheckSumGenerator(),
  });
});

// Given a workflow including 4 tasks
// When I execute 2 tasks successfully and third one fails
// Then I retry execution of the all tasks
// Then the library does not execute the first two tasks and successfully executes all tasks.

// Setup transformer with in-memory storage for testing
Given('a workflow including 4 tasks', async function () {
  runner = await transformer.startWorkflow(idempotentWorkflowKey, {
    contextName: 'workflow-with-4-tasks',
    isNested: false,
  });
});
When('I execute 2 tasks successfully and third one fails', async function () {
  let error: unknown;
  try {
    const result1 = await runner.execute('task1', async () => await workflowTasks.task1(input));
    const result2 = await runner.execute('task2', async () => await workflowTasks.task2(result1));
    await runner.execute('task3', async () => await workflowTasks.task3(result2));
    await runner.execute('task4', async () => await workflowTasks.task4(input));
    await runner.complete();
  } catch (err) {
    error = err;
  }
  expect(error).to.be.instanceOf(Error);
});

Then('I retry execution of the all tasks', async function () {
  ++retryCount;
  const newRunner = await transformer.startWorkflow(idempotentWorkflowKey, {
    contextName: 'workflow-with-4-tasks',
    isNested: false,
  });
  await newRunner.execute('task1', async () => await workflowTasks.task1(input));
  await newRunner.execute('task2', async () => await workflowTasks.task2(input));
  await newRunner.execute('task3', async () => await workflowTasks.task3(input));
  await newRunner.execute('task4', async () => await workflowTasks.task4(input));
  await newRunner.complete();
});

Then(
  'the task should execute successfully and all tasks should have been executed only once',
  async function () {
    console.log('workflowExecution', workflowExecution);
    expect(workflowExecution[0]).to.equal(1);
    expect(workflowExecution[1]).to.equal(1);
    expect(workflowExecution[2]).to.equal(1);
    expect(workflowExecution[3]).to.equal(1);
  }
);

AfterAll(async () => {});
