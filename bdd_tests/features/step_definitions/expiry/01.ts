import { Given, BeforeAll, Then, setDefaultTimeout } from '@cucumber/cucumber';
import { expect } from 'chai';
import { IdempotentRunnerResult, IdempotentTransformer } from '@idempotent-transformer/core';
import { MessagePack } from '@idempotent-transformer/message-pack-adapter';
import { faker } from '@faker-js/faker';
import { IdempotentFactory } from '@idempotent-transformer/core';
import { CheckSumGenerator } from '@idempotent-transformer/checksum-adapter';
import { GrpcAdapter } from '@idempotent-transformer/grpc-adapter';

setDefaultTimeout(15000);

let transformer: IdempotentTransformer;
const taskResult = faker.lorem.sentence();
const workflowId = faker.string.uuid();
const ttlMs = 3000; // 5 seconds
let rpcAdapter: GrpcAdapter;
let runner: IdempotentRunnerResult;

// Feature: Optional expiry (TTL) for state entries

//   Scenario: Persisting a task result with a TTL
//     Given a TTL of 5 seconds is configured for workflow completion data state
//     When a task is executed and workflow is completed
//     Then wait for 10 seconds and the workflow must not be found

BeforeAll(async () => {
  rpcAdapter = new GrpcAdapter({
    host: 'localhost',
    port: 51000,
  });
  transformer = await IdempotentFactory.getInstance().build({
    rpcAdapter,
    serializer: MessagePack.getInstance(),
    logger: null,
    checksumGenerator: new CheckSumGenerator(),
  });
});

Given('a TTL of 5 seconds is configured for workflow completion data state', async function () {
  runner = await transformer.startWorkflow(workflowId, {
    workflowName: 'workflow-with-4-tasks',
    completedRetentionTime: ttlMs,
  });
});

Given('a task is executed and workflow is completed', async function () {
  // Already set up in previous step
  await runner.execute('task', async () => taskResult);
  await runner.complete();
});

Then('wait for 10 seconds and the workflow must not be found', async function () {
  const persisted = await runner.getWorkflowStatus({
    workflowId,
  });
  expect(persisted?.id).to.equal(workflowId);
  await new Promise((resolve) => setTimeout(resolve, 12000));
  let error: Error | undefined;
  try {
    await runner.getWorkflowStatus({
      workflowId,
    });
  } catch (err) {
    error = err as Error;
  }
  expect(error?.message).to.equal('workflow_not_found');
});
