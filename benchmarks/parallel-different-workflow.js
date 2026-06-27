const IdempotentTransformer = require('@idempotent-transformer/core').IdempotentTransformer;
const GrpcAdapter = require('@idempotent-transformer/grpc-adapter').GrpcAdapter;
const MessagePack = require('@idempotent-transformer/message-pack-adapter').MessagePack;
const { faker } = require('@faker-js/faker');

const messagePack = MessagePack.getInstance();

const getData = () => ({
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
        quotes: { USD: faker.number.float({ min: 1000, max: 10000 }) },
      },
    },
    currency_rates: { ETH: { USD: faker.number.float({ min: 1000, max: 10000 }) } },
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
});

const STEPS = [
  ['task1', async () => getData()],
  ['task2', async () => getData()],
  ['task3', async () => getData()],
  ['task4', async () => getData()],
  ['task5', async () => getData()],
  ['task6', async () => getData()],
];

const BASE_PORT = 9000;
const transformer = new IdempotentTransformer({
  rpcAdapter: new GrpcAdapter({ host: 'localhost', port: BASE_PORT }),
  serializer: messagePack,
  log: null,
});

module.exports = {
  runWorkflow: async (context, events) => {
    const t0 = performance.now();

    const runner = await transformer.startWorkflow(context.vars.$uuid, {
      completedRetentionTime: 70_000,
      name: context.vars.$uuid,
    });
    events.emit('histogram', 'workflow.start_ms', performance.now() - t0);

    try {
      for (const [name, fn] of STEPS) {
        const ts = performance.now();
        await runner.execute(name, fn);
        events.emit('histogram', `workflow.${name}_ms`, performance.now() - ts);
      }

      const tc = performance.now();
      await runner.complete();
      events.emit('histogram', 'workflow.complete_ms', performance.now() - tc);
      events.emit('histogram', 'workflow.total_ms', performance.now() - t0);
      events.emit('counter', 'workflows.success', 1);
    } catch (err) {
      await runner.complete().catch(() => {});
      console.error(err, context.vars.$uuid);
      events.emit('counter', 'workflows.fail', 1);
    }
  },
};
