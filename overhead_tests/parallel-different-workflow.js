const IdempotentTransformer = require('@idempotent-transformer/core').IdempotentTransformer;
const GrpcAdapter = require('@idempotent-transformer/grpc-adapter').GrpcAdapter;
const MessagePack = require('@idempotent-transformer/message-pack-adapter').MessagePack;
const CheckSumGenerator = require('@idempotent-transformer/checksum-adapter').CheckSumGenerator;
const { faker } = require('@faker-js/faker');

const messagePack = MessagePack.getInstance();


const getData = () =>  {
  const testData = {
    callback_id: faker.string.uuid(),
    callback_status: 'transaction_complete',
    signature: faker.string.uuid(),
    transaction: {
      id: faker.string.uuid(),
      kind: faker.finance.currencyCode(),
      txid: faker.string.uuid(),
      invoice: {
        id: faker.string.uuid(),
        kind: faker.finance.currencyCode(),
        created_at: faker.date.past(),
        profile_id: faker.string.uuid(),
        address: faker.finance.ethereumAddress(),
        lightning: null,
        network: faker.finance.currencyCode(),
        amount: {
          requested: {
            amount: faker.number.float({ min: 1000, max: 10000 }),
            currency: faker.finance.currencyCode(),
          },
          invoiced: {
            amount: faker.number.float({ min: 0.001, max: 0.01 }),
            currency: faker.finance.currencyCode(),
          },
        },
        custom_fee: null,
        min_confirmations: null,
        zero_conf_enabled: null,
        notes: null,
        passthrough: JSON.stringify({
            type: 'deposit',
            depositId: faker.string.uuid(),
            tenantId: faker.string.uuid(),
        }),
      },
      amount: {
        paid: {
          amount: faker.number.float({ min: 0.001, max: 0.01 }),
          currency: faker.finance.currencyCode(),
          quotes: {
            USD: faker.number.float({ min: 1000, max: 10000 }),
          },
        },
      },
      currency_rates: {
        ETH: {
          USD: faker.number.float({ min: 1000, max: 10000 }),
        },
      },
      created_at: faker.date.past(),
      executed_at: faker.date.past(),
      confirmed_at: faker.date.past(),
      confirmations: faker.number.int(100),
      status: 'complete',
      zero_conf_status: null,
      network: faker.finance.ethereumAddress(),
      risk_level: null,
      risk_data: null,
    },
  };
  return testData;
}
// Helper function for waiting
const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// Task definitions - same as your experiment
const task1 = async () => {
  return getData();
};

const task2 = async () => {
  return getData();
};

const task3 = async () => {
  return getData();
};

const task4 = async () => {
  return getData();
};

const task5 = async () => {
  return getData();
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
      log: null,
    });
    const runner = await transformer.startWorkflow(context.vars.$uuid, {
      workflowName: context.vars.$uuid,
      completedRetentionTime: 10000,
    });
    try {
        await runner.execute('task1', task1);
        await runner.execute('task2', task2);
        await runner.execute('task3', task3);
        // await runner.execute('task4', task4);
        // await runner.execute('task5', task5);
        await runner.complete();
      events.emit('counter', 'workflows.success', 1);

    } catch (err) {
      await runner.complete();

      console.error(err);
      events.emit('counter', 'workflows.fail', 1);
    }
  },
};
