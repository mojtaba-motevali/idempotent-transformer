import { RedisAdapter } from '@idempotent-transformer/adapter-redis';
import { MessagePack } from '@idempotent-transformer/adapter-message-pack';
import { Md5Adapter } from '@idempotent-transformer/adapter-crypto';
import { IdempotentFactory, IdempotentTransformer } from '@idempotent-transformer/core';
import { faker } from '@faker-js/faker';
import Redis from 'ioredis';



const storage = new RedisAdapter({
    options: {
      host: 'localhost',
      port: 6379,
    },
  });

const crypto = new Md5Adapter();
const serializer = MessagePack.getInstance();

const redisRaw = new Redis({ host: 'localhost', port: 6379 });

// Baseline: store as plain JSON, no compression
async function storeBaseline(key, data) {
    const json = JSON.stringify(data);
    const inputHash= await crypto.createHash(json);
    // Same data shape. 
    const finalData = JSON.stringify({
      in: inputHash,
      re: json,
    });
    await redisRaw.set(key, finalData);
    return Buffer.byteLength(finalData);
  }

// Optimized: use IdempotentTransformer (MessagePack + Zstd)
async function storeOptimized(workflowId, data) {
  const idempotent = await IdempotentTransformer.getInstance().makeIdempotent(workflowId, {
    task: async (input) => data,
  });
  const taskId= await crypto.createHash(
   await serializer.serialize({
      workflowId,
      contextName: 'task',
    })
  );
  await idempotent.task(data);
  // Fetch raw bytes from Redis
  const redisKey = `idempotent-workflows:${taskId}`;
  const buf = await redisRaw.getBuffer(redisKey);
  return buf ? buf.length : 0;
}
async function run(workflowId) {

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
    // Store baseline
    const baselineKey = `baseline:${workflowId}`;
    const baselineSize = await storeBaseline(baselineKey, testData);
    // Store optimized
    const optimizedSize = await storeOptimized(workflowId, testData);
    // Log results
    const reduction = baselineSize && optimizedSize ? ((baselineSize - optimizedSize) / baselineSize) * 100 : 0;
    console.log(`Baseline size: ${baselineSize} bytes, Optimized size: ${optimizedSize} bytes, Reduction: ${reduction.toFixed(2)}%`);
    return {
        baselineSize,
        optimizedSize,
        reduction,
    }
}

(async () => {
    await IdempotentFactory.build({
        storage,
        serializer,
        logger: null,
        crypto,
      });
      const now = Date.now();
    console.log('Starting tests...');
    let totalBaselineSize = 0;
    let totalOptimizedSize = 0;
    for (let i = 0; i < 10000; i++) {
        const result = await run(i);
        totalBaselineSize += result.baselineSize;
        totalOptimizedSize += result.optimizedSize;
    }
    console.log(`Tests complete in ${Date.now() - now}ms`);
    console.log(`Total baseline size: ${totalBaselineSize}`);
    console.log(`Total optimized size: ${totalOptimizedSize}`);
    console.log(`Total reduction: ${((totalBaselineSize - totalOptimizedSize) / totalBaselineSize) * 100}%`);
})()