const IdempotentTransformer = require('@idempotent-transformer/core').IdempotentTransformer;
const GrpcAdapter = require('@idempotent-transformer/grpc-adapter').GrpcAdapter;
const MessagePack = require('@idempotent-transformer/message-pack-adapter').MessagePack;
const appendFile = require('fs').appendFile;

const messagePack = MessagePack.getInstance();

// Track task execution counts
const fileName = 'test.txt';

const appendFileAsync = async (fileName, content) =>
  new Promise((resolve, reject) => {
    appendFile(fileName, content, (err) => {
      if (err) {
        reject(err);
      }
      resolve();
    });
  });
// Helper function for waiting
const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// Task definitions - same as your experiment
const task1 = async () => {
  const now = Date.now();
  await wait(300);
  await appendFileAsync(fileName, now.toString() + '\n');
  return now.toString();
};

const task2 = async () => {
  const now = Date.now();
  await wait(300);
  await appendFileAsync(fileName, now.toString() + '\n');
  return now.toString();
};

const task3 = async () => {
  const now = Date.now();
  await wait(100);
  await appendFileAsync(fileName, now.toString() + '\n');
  return now.toString();
};

const task4 = async () => {
  const now = Date.now();
  await wait(500);
  await appendFileAsync(fileName, now.toString() + '\n');
  return now.toString();
};

const task5 = async () => {
  const now = Date.now();
  await wait(1000);
  appendFile(fileName, now.toString() + '\n', (err) => {
    if (err) {
      console.error(err);
    }
  });
  return now.toString();
};
const TOTAL_CLUSTER_NODES = 3;
// round-robin port selection to balance load across nodes
const BASE_PORT = 51000;
const nodePorts = Array.from({ length: TOTAL_CLUSTER_NODES }, (_, i) => BASE_PORT + i);
let nextNodeIndex = 0;
const getNextPort = () => {
  const port = nodePorts[nextNodeIndex];
  nextNodeIndex = (nextNodeIndex + 1) % nodePorts.length;
  return port;
};

const workflowId = '1000';

module.exports = {
  runWorkflow: async (context, events) => {
    const rpcAdapter = new GrpcAdapter({
      host: 'localhost',
      port: getNextPort(),
    });
    const transformer = new IdempotentTransformer({
      rpcAdapter,
      serializer: messagePack,
      log: null,
    });
    try {
      const runner = await transformer.startWorkflow(workflowId, {
        contextName: 'my-workflow',
      });

      await runner.execute('task1', task1);
      await runner.execute('task2', task2);
      await runner.execute('task3', task3);
      await runner.execute('task4', task4);
      await runner.execute('task5', task5);

      await runner.complete();
      events.emit('counter', 'workflows.success', 1);
    } catch (err) {
      console.error(err);
      events.emit('counter', 'workflows.fail', 1);
    }
  },
};
