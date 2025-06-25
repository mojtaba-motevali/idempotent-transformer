# Idempotent Transformer

A TypeScript library for creating and working with idempotent transformations. Idempotency is the property of certain operations in mathematics and computer science whereby they can be applied multiple times without changing the result beyond the initial application. This is a crucial concept in building robust and reliable systems, especially in distributed environments where requests might be duplicated.

## Table of Contents

- [Core Concepts](#core-concepts)
- [Packages](#packages)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Usage](#usage)
  - [1. Configure the IdempotentTransformer](#1-configure-the-idempotenttransformer)
  - [2. Get the transformer instance](#2-get-the-transformer-instance)
  - [3. Make a function idempotent](#3-make-a-function-idempotent)
  - [NestJS Integration](#nestjs-integration)
- [Testing](#testing)
  - [BDD Tests](#bdd-tests)
  - [Overhead Tests](#overhead-tests)
- [Examples](#examples)
- [Development](#development)
- [License](#license)

## Core Concepts

The library is built around a few core concepts:

- **IdempotentTransformer**: The main class that orchestrates the process of making functions idempotent.
- **StateStore**: A storage mechanism to keep track of function executions and their results. You can implement your own or use provided adapters.
- **Serializer**: A mechanism to serialize and deserialize data before storing it. This is important for complex data types.
- **Compressor**: An optional component to compress the serialized data to save storage space.
- **Crypto**: An optional component to hash the input to create a unique key for each execution.

## Packages

This is a monorepo containing several packages.

| Package                                        | Description                                                |
| ---------------------------------------------- | ---------------------------------------------------------- |
| `@idempotent-transformer/core`                 | The core library containing the main logic and interfaces. |
| `@idempotent-transformer/adapter-redis`        | An adapter for using Redis as a state store.               |
| `@idempotent-transformer/adapter-message-pack` | An adapter for using MessagePack as a serializer.          |
| `@idempotent-transformer/adapter-zstd`         | An adapter for using ZSTD for compression.                 |
| `@idempotent-transformer/adapter-crypto`       | An adapter for using MD5 for hashing.                      |
| `@idempotent-transformer/nestjs`               | A module for easy integration with the NestJS framework.   |

## Prerequisites

- TypeScript >= 5.0

## Installation

You need to install the core package and any adapters you want to use.

```bash
npm install @idempotent-transformer/core
npm install @idempotent-transformer/adapter-redis ioredis
npm install @idempotent-transformer/adapter-message-pack msgpackr
npm install @idempotent-transformer/adapter-zstd
npm install @idempotent-transformer/adapter-crypto
```

For NestJS integration:

```bash
npm install @idempotent-transformer/nestjs
```

## Usage

### 1. Configure the IdempotentTransformer

First, you need to configure the `IdempotentTransformer` using the `IdempotentFactory`.

```typescript
import { IdempotentFactory } from '@idempotent-transformer/core';
import { RedisAdapter } from '@idempotent-transformer/adapter-redis';
import { MessagePack } from '@idempotent-transformer/adapter-message-pack';
import { ZstdCompressor } from '@idempotent-transformer/adapter-zstd';
import { Md5Adapter } from '@idempotent-transformer/adapter-crypto';

async function configureTransformer() {
  await IdempotentFactory.build({
    storage: new RedisAdapter({ options: { host: 'localhost', port: 6379 } }),
    serializer: MessagePack.getInstance(),
    compressor: new ZstdCompressor(),
    crypto: new Md5Adapter(),
    log: console, // Optional logger
  });
}
```

### 2. Get the transformer instance

```typescript
import { IdempotentTransformer } from '@idempotent-transformer/core';

const transformer = IdempotentTransformer.getInstance();
```

### 3. Make a function idempotent

You can make any `async` function or a set of functions idempotent using `makeIdempotent`.

```typescript
class PaymentService {
  async processPayment(orderId: string, amount: number): Promise<{ status: string }> {
    // Actual payment processing logic
    console.log(`Processing payment for order ${orderId} with amount ${amount}`);
    return { status: 'completed' };
  }
}

const paymentService = new PaymentService();

const idempotentPaymentService = await transformer.makeIdempotent(
  'payment-workflow', // A unique ID for this workflow
  {
    processPayment: (...args: Parameters<typeof paymentService.processPayment>) =>
      paymentService.processPayment(...args),
  }
);

// The first call will execute the function
const result1 = await idempotentPaymentService.processPayment('order-123', 100);
console.log(result1); // { status: 'completed' } (from execution)

// Subsequent calls with the same arguments will return the cached result
const result2 = await idempotentPaymentService.processPayment('order-123', 100);
console.log(result2); // { status: 'completed' } (from cache)
```

### NestJS Integration

The `@idempotent-transformer/nestjs` package provides a `IdempotentModule` for easy integration.

In your `app.module.ts`:

```typescript
import { Module } from '@nestjs/common';
import { IdempotentModule } from '@idempotent-transformer/nestjs';
import { RedisAdapter } from '@idempotent-transformer/adapter-redis';
import { MessagePack } from '@idempotent-transformer/adapter-message-pack';
import { ZstdCompressor } from '@idempotent-transformer/adapter-zstd';
import { Md5Adapter } from '@idempotent-transformer/adapter-crypto';

@Module({
  imports: [
    IdempotentModule.registerAsync({
      useFactory: () => {
        return {
          storage: new RedisAdapter({ options: { host: 'localhost', port: 6379 } }),
          serializer: MessagePack.getInstance(),
          compressor: new ZstdCompressor(),
          crypto: new Md5Adapter(),
        };
      },
    }),
  ],
})
export class AppModule {}
```

Then you can inject the `IdempotentTransformer` in your services:

```typescript
import { Injectable, Inject } from '@nestjs/common';
import { IdempotentTransformer, IDEMPOTENT_TRANSFORMER } from '@idempotent-transformer/nestjs';

@Injectable()
export class AppService {
  constructor(
    @Inject(IDEMPOTENT_TRANSFORMER) private readonly transformer: IdempotentTransformer
  ) {}

  // ... use the transformer
}
```

## Testing

### BDD Tests

To run the BDD tests, navigate to the `bdd_tests` directory and run the tests:

```bash
cd bdd_tests
npm install
npm test
```

### Overhead Tests

These tests measure the storage size impact. You'll need a running Redis instance to execute them.

To run the overhead tests:

```bash
# Make sure redis is running
node overhead_tests/experiments/[test_name]
```

Replace `[test_name]` with the experiment file you want to run (e.g., `compression-disabled-msg-json.experiment.js`).

## Examples

This repository includes examples for different frameworks:

- **nestjs**: A complete NestJS application showing how to integrate the transformer.
- **nextjs**: A Next.js application demonstrating usage in a React-based framework.

## Development

```bash
pnpm install
pnpm build:all
```

## License

MIT
