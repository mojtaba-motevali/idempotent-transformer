import { Given, When, Then, BeforeAll, AfterAll } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentTransformer } from '../../../../../lib/idempotent-transformer';
import { IOptions } from '../../../../../lib/idempotent-transformer/interfaces/idempotent-options.interface';
import { ConsoleLogger } from '../../../../../lib/logger/console-logger';
import { Repository } from '../../../../../adapters/redis';
import { MessagePack } from '../../../../../adapters/message-pack';
import { ZstdCompressor } from '../../../../../adapters/zstd';
import { faker } from '@faker-js/faker';
import { IdempotencyResult } from '../../../../../lib/idempotent-transformer/interfaces/idempotency-result.interface';

let transformer: IdempotentTransformer;
let wrappedTask: (input: string, input2: number, options?: IOptions) => Promise<any>;
let storage: Repository;
const taskInput = faker.lorem.sentence();
const taskInput2 = faker.number.int();
const taskResult = faker.lorem.sentence();
let taskUniqueId: string;
const workflowId = faker.string.uuid();
const compressor = new ZstdCompressor();

BeforeAll(async () => {
  storage = new Repository('redis://localhost:6379');
  await storage.initialize();
  transformer = IdempotentTransformer.getInstance({
    storage,
    serializer: MessagePack.getInstance(),
    compressor,
    log: new ConsoleLogger(),
  });
});

Given('compression is enabled', async function () {
  // Already set in BeforeAll
});

Given('a task result "Hello, world!" is ready to be persisted', async function () {
  const asyncTask = async (input: string, input2: number) => taskResult;
  const wrapped = transformer.makeIdempotent(workflowId, { task: asyncTask });
  wrappedTask = wrapped.task;
});

When('the task result is serialized and persisted to the state store', async function () {
  // Persist the result with compression enabled
  await wrappedTask(taskInput, taskInput2, { shouldCompress: true });
  // Compute the taskUniqueId for direct state store access
  taskUniqueId = await transformer.createHash({
    workflowId,
    functionName: 'task',
  });
});

Then('the persisted data should be compressed', async function () {
  const persisted = await storage.find(taskUniqueId);
  // Try to decompress, should succeed
  let decompressed: Uint8Array | undefined = undefined;
  let decompressError = null;
  try {
    decompressed = await compressor.decompress(persisted!);
  } catch (err) {
    decompressError = err;
  }
  // Check that the decompression succeeded, this throws an exception if it fails to decompress.
  expect(decompressError).to.be.null;
  expect(decompressed).to.not.be.undefined;
});

Then('the original task result should not be stored in plain text', async function () {
  const persisted = await storage.find(taskUniqueId);
  let deserialized: IdempotencyResult<string> | undefined;
  let error = null;
  try {
    deserialized = await MessagePack.getInstance().deserialize(persisted as Uint8Array);
  } catch (err) {
    error = err;
  }
  expect(deserialized).to.be.undefined;
  expect(error).to.not.be.null;
});

AfterAll(async () => {
  await storage.destroy();
});
