import { Given, When, Then, Before } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentTransformer } from '../../../../lib/idempotent-transformer';
import { IdempotencyKey } from '../../../../lib/idempotent-transformer/interfaces/idempotent-key.interface';
import { Options } from '../../../../lib/idempotent-transformer/interfaces/idempotent-options.interface';
import { ConsoleLogger } from '../../../../lib/logger/console-logger';
import { Repository } from '../../../../adaptors/redis';
import { MessagePack } from '../../../../adaptors/message-pack';
import { ZstdCompressor } from '../../../../adaptors/zstd';

let transformer: IdempotentTransformer;
let wrappedTask: (input: any, key: IdempotencyKey, options?: Options) => Promise<any>;
let asyncTask: (input: any) => Promise<any>;
let firstResult: any;
let secondResult: any;
let taskExecutionCount = 0;

Before(async () => {
  const storage = new Repository('redis://localhost:6379');
  await storage.initialize();

  transformer = IdempotentTransformer.getInstance({
    storage,
    serializer: new MessagePack(),
    compressor: new ZstdCompressor(),
    log: new ConsoleLogger(),
  });
});

// Setup transformer with in-memory storage for testing
Given('an asynchronous task that returns a value', async function () {
  asyncTask = async (input: any) => {
    taskExecutionCount++;
    return {
      a: `Processed: ${input.input}`,
      b: `Processed: ${input.input}`,
    };
  };
});
When('I wrap the task with the idempotent execution wrapper', async function () {
  const wrapped = transformer.makeIdempotent('test-workflow', { task: asyncTask });
  wrappedTask = wrapped.task;
});

When('I execute the wrapped task', async function () {
  firstResult = await wrappedTask(
    {
      input: 'test-input',
    },
    { key: 'test-key' }
  );
});

Then('the task should execute successfully and the result should be returned', async function () {
  expect(firstResult.a).to.equal('Processed: test-input');
  expect(firstResult.b).to.equal('Processed: test-input');
});

When('I execute the wrapped task again', async function () {
  secondResult = await wrappedTask(
    {
      input: 'test-input',
    },
    { key: 'test-key' }
  );
});

Then(
  'the task should not be executed again and the cached result should be returned',
  async function () {
    expect(secondResult).to.deep.equal(firstResult);
    expect(taskExecutionCount).to.equal(1);
  }
);
