// import { Given, When, Then, BeforeAll, AfterAll } from '@cucumber/cucumber';
// import { expect } from 'chai';
// import { IdempotentTransformer, IIdempotentTaskOptions } from '@idempotent-transformer/core';
// import { PostgresAdapter } from '@idempotent-transformer/postgres-adapter';
// import { MessagePack } from '@idempotent-transformer/message-pack-adapter';
// import { faker } from '@faker-js/faker';
// import { IdempotentFactory } from '@idempotent-transformer/core';
// import { CheckSumGenerator } from '@idempotent-transformer/checksum-adapter';

// let wrappedTask: (input: any, options?: IIdempotentTaskOptions) => Promise<any>;
// let storage: PostgresAdapter;
// let transformer: IdempotentTransformer;
// const taskInput = faker.lorem.sentence();
// const taskResult = faker.lorem.sentence();
// let taskUniqueId: string;
// const workflowId = faker.string.uuid();
// const ttlMs = 3000; // 30 seconds

// BeforeAll(async () => {
//   storage = new PostgresAdapter('postgres://postgres:postgres@localhost:5432/postgres');
//   transformer = await IdempotentFactory.getInstance().build({
//     storage,
//     serializer: MessagePack.getInstance(),
//     logger: null,
//     checksumGenerator: new CheckSumGenerator(),
//   });
// });

// Given('a TTL of 2 seconds is configured for state entries S1', async function () {
//   const wrapped = await transformer.makeIdempotent(
//     workflowId,
//     {
//       task: async (input: any) => taskResult,
//     },
//     {
//       retentionTime: ttlMs,
//     }
//   );
//   wrappedTask = wrapped.task;
// });

// Given('a task result "Hello, world!" is ready to be persisted S1', async function () {
//   // Already set up in previous step
// });

// When('the task result is serialized and persisted to the state store S1', async function () {
//   await wrappedTask(taskInput);
//   taskUniqueId = (
//     await transformer.createCheckSum({
//       workflowId,
//       contextName: 'task',
//     })
//   ).toString();
// });

// Then('the persisted entry should exist in the state store immediately S1', async function () {
//   const persisted = await storage.find({
//     workflowId,
//     taskId: taskUniqueId,
//   });
//   expect(persisted).to.not.be.null;
// });

// Then('after 3 seconds, the entry should be expired and no longer available S1', async function () {
//   await new Promise((resolve) => setTimeout(resolve, 3000));
//   const persisted = await storage.find({
//     workflowId,
//     taskId: taskUniqueId,
//   });
//   expect(persisted).to.be.null;
// });

// AfterAll(async () => {
//   await storage.disconnect();
// });
