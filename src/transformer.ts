import { IdempotentTransformer } from './lib/idempotent-transformer/idempotent-transformer';

(async () => {
  const logic = new IdempotentTransformer({
    log: (message: string, level: 'info' | 'error' | 'warn' | 'debug') => {
      console.log(`${level}: ${message}`);
    },
    storage: 'postgres',
    broker: {
      publish: async (topic: string, message: string) => {
        console.log(`Publishing message to topic: ${topic}`);
      },
      subscribe: async (topic: string, callback: (message: string) => void) => {
        console.log(`Subscribing to topic: ${topic}`);
      },
    },
  }).makeIdempotent({
    double: async (x: number) => x * 2,
    square: async (x: number) => x * x,
    increment: async (x: number) => x + 1,
  });

  const result = await logic.double(
    2,
    { id: '2' },
    { shouldEncrypt: true, shouldCacheInput: true, shouldCacheOutput: true, shouldCompress: true }
  );

  console.log(`double: ${result}`);

  const result2 = await logic.square(
    3,
    { id: '3' },
    { shouldEncrypt: true, shouldCacheInput: true, shouldCacheOutput: true, shouldCompress: true }
  );

  console.log(`square: ${result2}`);

  const result3 = await logic.increment(
    4,
    { id: '4' },
    { shouldEncrypt: true, shouldCacheInput: true, shouldCacheOutput: true, shouldCompress: true }
  );

  console.log(`increment: ${result3}`);
})();
