import { Given, When, Then, AfterAll, BeforeAll } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentTransformer } from '../../../../../packages/core/idempotent-transformer';
import { RedisAdapter } from '../../../../../packages/adapter-redis/redis';
import { MessagePack } from '../../../../../packages/adapter-message-pack/message-pack';
import { ZstdCompressor } from '../../../../../packages/adapter-zstd/zstd-compressor';
import { faker } from '@faker-js/faker';
import { IdempotentFactory } from '../../../../../packages/core/factory/idempotent-factory';

let retryCount = 0;
let workflowExecution = [0, 0, 0, 0, 0, 0, 0];

const input = faker.string.uuid();
const idempotentWorkflowKey = faker.string.uuid();
const storage: RedisAdapter = new RedisAdapter('redis://localhost:6379');

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
  await IdempotentFactory.build({
    storage,
    serializer: MessagePack.getInstance(),
    compressor: new ZstdCompressor(),
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
        const { task5, task6, task7 } = IdempotentTransformer.getInstance().makeIdempotent(
          idempotentWorkflowKey,
          {
            task5: innerWorkflowTasks.task5,
            task6: innerWorkflowTasks.task6,
            task7: innerWorkflowTasks.task7,
          }
        );
        const result = await task5({
          shouldCompress: true,
        });
        const result2 = await task6(result);
        const result3 = await task7(result2);
        ++workflowExecution[1];
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
  const wrapped = IdempotentTransformer.getInstance().makeIdempotent(idempotentWorkflowKey, {
    task1: outerWorkflowTasks.task1,
    task3: outerWorkflowTasks.task3,
    task4: outerWorkflowTasks.task4,
  });
  let error: unknown;
  try {
    const result1 = await wrapped.task1();
    const result2 = await outerWorkflowTasks.task2();
    await wrapped.task3(result2);
    await wrapped.task4(result2);
  } catch (err) {
    error = err;
  }
  expect(error).to.be.instanceOf(Error);
});

Then('I retry the outer workflow to recover from the failure', async function () {
  ++retryCount;
  const wrapped = IdempotentTransformer.getInstance().makeIdempotent(idempotentWorkflowKey, {
    task1: outerWorkflowTasks.task1,
    task3: outerWorkflowTasks.task3,
    task4: outerWorkflowTasks.task4,
  });
  await wrapped.task1();
  const result2 = await outerWorkflowTasks.task2();
  await wrapped.task3(result2);
  await wrapped.task4(result2);
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

AfterAll(async () => {
  await storage.disconnect();
});
