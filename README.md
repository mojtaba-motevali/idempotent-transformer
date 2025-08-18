# Idempotent Transformer

A comprehensive TypeScript/JavaScript library for implementing idempotent operations with distributed workflow support, featuring a high-performance Rust server backend.

## Overview

Idempotent Transformer provides a robust solution for ensuring that operations are executed exactly once, even in distributed systems with potential failures and retries. It consists of:

- **Core Library**: TypeScript/JavaScript library for wrapping functions with idempotency guarantees
- **Rust Server**: High-performance gRPC server for distributed workflow management
- **Framework Adapters**: Ready-to-use integrations for NestJS, Next.js, and other frameworks

## Features

- 🔄 **True Idempotency**: Guarantees operations execute exactly once
- 🏗️ **Workflow Support**: Manage complex multi-step workflows with checkpointing
- 🚀 **High Performance**: Rust backend with async/await support
- 🔌 **Framework Agnostic**: Works with any JavaScript/TypeScript framework
- 🎯 **Type Safe**: Full TypeScript support with comprehensive type definitions
- 🧪 **BDD Testing**: Behavior-driven development tests included
- 📦 **Modular Design**: Pluggable adapters for RPC communication and serialization

## Architecture

The system consists of two main components:

1. **Client Library** (`packages/core`): TypeScript library that manages workflows and checkpoints
2. **Server** (`server/`): Rust gRPC server for distributed state management

```
┌─────────────────┐    gRPC     ┌─────────────────┐
│   Your App      │ ──────────► │  Rust Server    │
│  (TypeScript)   │             │  (Workflow      │
│                 │             │   Management)   │
└─────────────────┘             └─────────────────┘
```

## Quick Start

### 1. Install the Core Package

```bash
npm install @idempotent-transformer/core @idempotent-transformer/grpc-adapter @idempotent-transformer/message-pack-adapter protobufjs
```

### 2. Set Up the Server

Build and run the Rust server using Cargo:

```bash
# Build the server
cd server
cargo run --release
```

Build and run the Rust server using Docker:

```bash
# Build the server
cd server
docker build -t idempotency-server .

# Run with Docker Compose (recommended for production)
docker-compose up
```

Or run a single node:

```bash
docker run -p 51000:51000 -p 8100:8100 -p 8200:8200 \
  -e RPC_ADDR="0.0.0.0:51000" \
  -e NODES="1 localhost:8100 localhost:8200" \
  -e DATA_DIR="/app/data" \
  -e NODE_ID="1" \
  -e ADDR_API="localhost:8100" \
  -e ADDR_RAFT="localhost:8200" \
  -v "/dev/shm/data/node_1:/app/data" \
  idempotency-server
```

### 3. Configure the Client

```typescript
import { IdempotentFactory } from '@idempotent-transformer/core';
import { GrpcAdapter } from '@idempotent-transformer/grpc-adapter';
import { MessagePack } from '@idempotent-transformer/message-pack-adapter';

async function initialize() {
  const rpcAdapter = new GrpcAdapter({
    host: 'localhost',
    port: 51000,
  });

  const transformer = await IdempotentFactory.getInstance().build({
    rpcAdapter,
    serializer: MessagePack.getInstance(),
    logger: null,
  });

  return transformer;
}

// Initialize once at app startup
const transformer = await initialize();
```

### 4. Use Workflow-Based Idempotent Functions

```typescript
import { IdempotentTransformer } from '@idempotent-transformer/core';

// Start a workflow
const runner = await transformer.startWorkflow('payment-workflow', {
  name: 'process-payment',
  completedRetentionTime: 24 * 60 * 60 * 1000, // 24 hours in milliseconds
});

// Execute tasks within the workflow
const result = await runner.execute('payment-task', async () => {
  // Your business logic here
  return await processPayment(paymentData);
});

// Complete the workflow
await runner.complete();
```

## Packages

### Core Package (`@idempotent-transformer/core`)

The main library providing the `IdempotentTransformer` class and workflow management.

```typescript
import { IdempotentTransformer, IdempotentFactory } from '@idempotent-transformer/core';
```

### gRPC Adapter (`@idempotent-transformer/grpc-adapter`)

Connects to the Rust server for distributed workflow management.

```typescript
import { GrpcAdapter } from '@idempotent-transformer/grpc-adapter';

const adapter = new GrpcAdapter({ host: 'localhost', port: 51000 });
```

### Message Pack Adapter (`@idempotent-transformer/message-pack-adapter`)

Efficient binary serialization using MessagePack.

```typescript
import { MessagePack } from '@idempotent-transformer/message-pack-adapter';

const serializer = MessagePack.getInstance();
```

### NestJS Module (`@idempotent-transformer/nestjs`)

Seamless integration with NestJS applications.

```typescript
import { IdempotentModule } from '@idempotent-transformer/nestjs';

@Module({
  imports: [
    IdempotentModule.registerAsync({
      useFactory: () => ({
        rpcAdapter: new GrpcAdapter({ host: 'localhost', port: 51000 }),
        serializer: MessagePack.getInstance(),
        logger: console,
      }),
    }),
  ],
})
export class AppModule {}
```

## Workflow Management

### Basic Workflow Usage

```typescript
// Start a workflow
const runner = await transformer.startWorkflow('unique-workflow-id', {
  name: 'workflow-name',
  completedRetentionTime: 60 * 60 * 1000, // 1 hour in milliseconds
});

// Execute tasks with idempotency
const result1 = await runner.execute('task1', async () => {
  return await performTask1();
});

const result2 = await runner.execute('task2', async () => {
  return await performTask2(result1);
});

// Complete the workflow
await runner.complete();
```

### Nested Workflows

```typescript
// Outer workflow
const outerRunner = await transformer.startWorkflow('outer-workflow', {
  name: 'outer-workflow',
  completedRetentionTime: 2 * 60 * 60 * 1000, // 2 hours in milliseconds
});

await outerRunner.execute('outer-task1', async () => {
  // Inner workflow
  const innerRunner = await transformer.startWorkflow('inner-workflow', {
    name: 'inner-workflow',
    completedRetentionTime: 30 * 60 * 1000, // 30 minutes in milliseconds
  });

  await innerRunner.execute('inner-task1', async () => {
    return await performInnerTask();
  });

  await innerRunner.complete();
});

await outerRunner.complete();
```

### Error Handling and Recovery

```typescript
try {
  const runner = await transformer.startWorkflow('workflow-id', {
    name: 'workflow-name',
    completedRetentionTime: 24 * 60 * 60 * 1000, // 24 hours in milliseconds
  });

  await runner.execute('task1', async () => await task1());
  await runner.execute('task2', async () => await task2());
  await runner.execute('task3', async () => await task3());
  await runner.complete();
} catch (error) {
  // On retry, completed tasks won't re-execute
  const runner = await transformer.startWorkflow('workflow-id', {
    name: 'workflow-name',
    completedRetentionTime: 24 * 60 * 60 * 1000, // 24 hours in milliseconds
  });

  // Only failed tasks will execute
  await runner.execute('task1', async () => await task1());
  await runner.execute('task2', async () => await task2());
  await runner.execute('task3', async () => await task3());
  await runner.complete();
}
```

## Server Configuration

### Environment Variables

| Variable    | Description            | Default                   |
| ----------- | ---------------------- | ------------------------- |
| `RPC_ADDR`  | gRPC service address   | `0.0.0.0:51000`           |
| `NODES`     | Cluster member list    | `1 node1:8100 node1:8200` |
| `DATA_DIR`  | Data directory         | `/app/data`               |
| `NODE_ID`   | Node identifier        | `1`                       |
| `ADDR_API`  | API address            | `0.0.0.0:8100`            |
| `ADDR_RAFT` | Raft consensus address | `0.0.0.0:8200`            |

### Docker Compose Setup

The included `docker-compose.yaml` sets up a 3-node cluster:

```bash
cd server
docker-compose up
```

This creates:

- 3 server nodes (node1, node2, node3)
- Nginx load balancer
- Persistent volumes for each node

### Single Node Development

For development, you can run a single node:

```bash
cd server
docker build -t idempotency-server .
docker run -p 51000:51000 idempotency-server
```

## Advanced Usage

### Custom Serialization

Use the `@Serialize` decorator for complex objects:

```typescript
import { Serialize } from '@idempotent-transformer/core';

@Serialize({
  name: 'User',
  serializeMethodName: 'serialize',
  deserializeMethodName: 'deserialize',
})
class User {
  #id: string;
  #name: string;

  constructor(id: string, name: string) {
    this.#id = id;
    this.#name = name;
  }

  serialize(): { id: string; name: string } {
    return { id: this.#id, name: this.#name };
  }

  static deserialize(data: { id: string; name: string }): User {
    return new User(data.id, data.name);
  }
}
```

### Workflow Options

#### completedRetentionTime

The `completedRetentionTime` option specifies how long (in milliseconds) the workflow state should be retained after completion. This is useful for:

- **Audit trails**: Keep workflow data for compliance requirements
- **Debugging**: Retain completed workflows for troubleshooting
- **Resource management**: Automatically clean up old workflow data

```typescript
// Retain workflow data for 24 hours after completion
const runner = await transformer.startWorkflow('workflow-id', {
  name: 'workflow-name',
  completedRetentionTime: 24 * 60 * 60 * 1000, // 24 hours
});

// Retain workflow data for 30 minutes after completion
const runner = await transformer.startWorkflow('workflow-id', {
  name: 'workflow-name',
  completedRetentionTime: 30 * 60 * 1000, // 30 minutes
});

// Default retention (24 hours) if not specified
const runner = await transformer.startWorkflow('workflow-id', {
  name: 'workflow-name',
});
```

### Task Options

```typescript
await runner.execute(
  'task-name',
  async () => {
    return await performTask();
  },
  {
    leaseTimeout: 30000, // 30 seconds lease timeout
  }
);
```

## Testing

The project includes comprehensive BDD tests:

```bash
cd bdd_tests
npm install
npm test
```

Test scenarios cover:

- Basic idempotent execution
- Workflow recovery and checkpointing
- Nested workflows
- Contract-based serialization
- Expiry handling

## Examples

### NestJS Example

See `examples/nestjs/` for a NestJS service with:

- gRPC adapter
- MessagePack serialization
- Dependency injection

## Performance

The Rust server is optimized for high performance:

- Async/await with Tokio runtime
- Jemalloc memory allocator
- gRPC for efficient communication
- Raft consensus for strong consistency

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.
