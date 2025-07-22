import { Given, When, Then, AfterAll, BeforeAll } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentRunnerResult, IdempotentTransformer } from '@idempotent-transformer/core';
import { MessagePack } from '@idempotent-transformer/message-pack-adapter';
import { faker } from '@faker-js/faker';
import { IdempotentFactory } from '@idempotent-transformer/core';
import { CheckSumGenerator } from '@idempotent-transformer/checksum-adapter';
import { GrpcAdapter } from '@idempotent-transformer/grpc-adapter';

let retryCount = 0;
let workflowExecution = [0, 0, 0, 0, 0, 0, 0];

const input = faker.string.uuid();
const idempotentWorkflowKey = faker.string.uuid();
const rpcAdapter: GrpcAdapter = new GrpcAdapter({
  host: 'localhost',
  port: 51000,
});
let transformer: IdempotentTransformer;
let runner: IdempotentRunnerResult;
let innerWorkflowTasks: {
  task5: () => Promise<any>;
  task6: () => Promise<any>;
  task7: () => Promise<any>;
};
let outerWorkflowTasks: {
  task1: () => Promise<any>;
  task2: () => Promise<any>;
  task3: () => Promise<any>;
  task4: () => Promise<any>;
};

BeforeAll(async () => {
  transformer = await IdempotentFactory.getInstance().build({
    rpcAdapter,
    serializer: MessagePack.getInstance(),
    checksumGenerator: new CheckSumGenerator(),
    logger: null,
  });
});

Given(
  'a workflow that includes multiple tasks that one of the includes multiple other tasks',
  async function () {
    innerWorkflowTasks = {
      task5: async () => {
        ++workflowExecution[4];
        return input;
      },
      task6: async () => {
        if (retryCount < 1) throw new Error('Task 6 failed');
        ++workflowExecution[5];
        return input;
      },
      task7: async () => {
        ++workflowExecution[6];
        return input;
      },
    };
    outerWorkflowTasks = {
      task1: async () => {
        ++workflowExecution[0];
        return input;
      },
      task2: async () => {
        const innerRunner = await transformer.startWorkflow(`${idempotentWorkflowKey}-task2`, {
          workflowName: 'inner workflow with 3 tasks',
        });
        const result = await innerRunner.execute('task5', async () => innerWorkflowTasks.task5());
        const result2 = await innerRunner.execute('task6', async () => innerWorkflowTasks.task6());
        const result3 = await innerRunner.execute('task7', async () => innerWorkflowTasks.task7());
        ++workflowExecution[1];
        await innerRunner.complete();
        return result + result2 + result3;
      },
      task3: async () => {
        ++workflowExecution[2];
      },
      task4: async () => {
        ++workflowExecution[3];
        return input;
      },
    };
    return undefined;
  }
);
When('I execute second task of the inner workflow and fails', async function () {
  let error: unknown;
  try {
    runner = await transformer.startWorkflow(idempotentWorkflowKey, {
      workflowName: 'a workflow with 4 tasks',
    });
    const result1 = await runner.execute('task1', async () => outerWorkflowTasks.task1());
    const result2 = await runner.execute('task2', async () => outerWorkflowTasks.task2());
    await runner.execute('task3', async () => outerWorkflowTasks.task3());
    await runner.execute('task4', async () => outerWorkflowTasks.task4());
    await runner.complete();
  } catch (err) {
    error = err;
  }
  expect(error).to.be.instanceOf(Error);
});

Then('I retry the outer workflow to recover from the failure', async function () {
  runner = await transformer.startWorkflow(idempotentWorkflowKey, {
    workflowName: 'a workflow with 4 tasks',
  });
  ++retryCount;

  await runner.execute('task1', async () => outerWorkflowTasks.task1());
  await runner.execute('task2', async () => outerWorkflowTasks.task2());
  await runner.execute('task3', async () => outerWorkflowTasks.task3());
  await runner.execute('task4', async () => outerWorkflowTasks.task4());
  await runner.complete();
});

Then(
  'the already successfully executed tasks must not re-execute and outer workflow must successfully finish.',
  async function () {
    expect(workflowExecution[0]).to.equal(1);
    expect(workflowExecution[1]).to.equal(1);
    expect(workflowExecution[2]).to.equal(1);
    expect(workflowExecution[3]).to.equal(1);
    expect(workflowExecution[4]).to.equal(1);
    expect(workflowExecution[5]).to.equal(1);
    expect(workflowExecution[6]).to.equal(1);
  }
);

AfterAll(async () => {});
