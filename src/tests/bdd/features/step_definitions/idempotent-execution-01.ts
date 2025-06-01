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
let asyncTask: (input: any) => Promise<any>;
let firstResult: any;
let secondResult: any;
let input = faker.string.uuid();
let storage: Repository;
const idempotencyKey = faker.string.uuid();
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

// Setup transformer with in-memory storage for testing
Given('an asynchronous task that returns a value', async function () {
  asyncTask = async (input: any) => {
    taskExecutionCount++;
    return {
      a: {
        b: {
          c: [input.input, input.input, input.input],
          d: null,
        },
        e: undefined,
        f: new Map().set('a', 'b'),
        g: new Set([1, 2, 3]),
        h: currentDate,
      },
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
      input,
    },
    { key: idempotencyKey }
  );
});

Then('the task should execute successfully and the result should be returned', async function () {
  expect(firstResult.a.b.c).to.deep.equal([input, input, input]);
  expect(firstResult.a.b.d).to.equal(null);
  expect(firstResult.a.e).to.equal(undefined);
  expect(firstResult.a.f.get('a')).to.equal('b');
  expect(firstResult.a.g.has(3)).to.equal(true);
  expect(firstResult.a.h).to.deep.equal(currentDate);
});

When('I execute the wrapped task again', async function () {
  secondResult = await wrappedTask(
    {
      input,
    },
    { key: idempotencyKey }
  );
});

Then(
  'the task should not be executed again and the cached result should be returned',
  async function () {
    expect(secondResult.a.b.c).to.deep.equal([input, input, input]);
    expect(secondResult.a.b.d).to.equal(null);
    expect(secondResult.a.e).to.equal(undefined);
    expect(secondResult.a.f.get('a')).to.equal('b');
    expect(secondResult.a.g.has(3)).to.equal(true);
    expect(secondResult.a.h).to.deep.equal(currentDate);
    expect(taskExecutionCount).to.equal(1);
  }
);

AfterAll(async () => {
  await storage.destroy();
});
