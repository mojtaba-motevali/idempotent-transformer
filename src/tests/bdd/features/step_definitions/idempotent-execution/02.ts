import { AfterAll, BeforeAll, Given, Then, When } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentTransformer } from '../../../../../lib/idempotent-transformer';
import { ConsoleLogger } from '../../../../../lib/logger/console-logger';
import { Repository } from '../../../../../adapters/redis';
import { MessagePack } from '../../../../../adapters/message-pack';
import { ZstdCompressor } from '../../../../../adapters/zstd';
import { faker } from '@faker-js/faker';
import { IdempotencyConflictException } from '../../../../../lib/idempotent-transformer/exceptions/conflict.exception';

let transformer: IdempotentTransformer;
let asyncTask: (input: any) => Promise<any>;
const workflowId = faker.string.uuid();
const input = faker.string.uuid();
const storage = new Repository('redis://localhost:6379');

BeforeAll(async () => {
  await storage.initialize();

  transformer = IdempotentTransformer.getInstance({
    storage,
    serializer: MessagePack.getInstance(),
    compressor: new ZstdCompressor(),
    log: new ConsoleLogger(),
  });
});

Given('an asynchronous task that accepts parameters', async function () {
  asyncTask = async (input: any) => {
    return {
      a: input,
    };
  };
});

When(
  'I wrap the task with the idempotent execution wrapper and I execute the wrapped task with parameter "A" successfully',
  async () => {
    const wrapped = transformer.makeIdempotent(workflowId, { task: asyncTask });
    await wrapped.task({
      input,
    });
  }
);

Then(
  'the task fails with Conflict exception when I execute the wrapped task with parameter "B"',
  async () => {
    const wrapped = transformer.makeIdempotent(workflowId, { task: asyncTask });
    let error: unknown;
    try {
      await wrapped.task({
        input: faker.string.uuid(),
      });
    } catch (err) {
      error = err;
    }
    expect(error).to.be.instanceOf(IdempotencyConflictException);
  }
);

AfterAll(async () => {
  await storage.destroy();
});
