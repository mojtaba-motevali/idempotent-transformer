const IdempotentTransformer = require('@idempotent-transformer/core').IdempotentTransformer;
const GrpcAdapter = require('@idempotent-transformer/grpc-adapter').GrpcAdapter;
const MessagePack = require('@idempotent-transformer/message-pack-adapter').MessagePack;
const CheckSumGenerator = require('@idempotent-transformer/checksum-adapter').CheckSumGenerator;
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
  return now.toString();
};

const task2 = async () => {
  const now = Date.now();
  return now.toString();
};

const task3 = async () => {
  const now = Date.now();
  return now.toString();
};

const task4 = async () => {
  const now = Date.now();
  return now.toString();
};

const task5 = async () => {
  const now = Date.now();

  return now.toString();
};

const TOTAL_CLUSTER_NODES = 1;
// round-robin port selection to balance load across nodes
const BASE_PORT = 51000;
const nodePorts = Array.from({ length: TOTAL_CLUSTER_NODES }, (_, i) => BASE_PORT + i);
let nextNodeIndex = 0;
const getNextPort = () => {
  const port = nodePorts[nextNodeIndex];
  nextNodeIndex = (nextNodeIndex + 1) % nodePorts.length;
  return port;
};

module.exports = {
  runWorkflow: async (context, events) => {
    const rpcAdapter = new GrpcAdapter({
      host: 'localhost',
      port: getNextPort(),
    });
    const transformer = new IdempotentTransformer({
      rpcAdapter,
      serializer: messagePack,
      checksumGenerator: new CheckSumGenerator(),
      log: null
    });
    try {
      const runner = await transformer.startWorkflow(context._uid, {
        contextName: context,
        retentionTime: 10000,
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
