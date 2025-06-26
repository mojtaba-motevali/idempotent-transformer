# @idempotent-transformer/core

For more comprehensive documentation, please see the main [README](https://github.com/mojtaba-motevali/idempotent-transformer).

This is the core package for the `idempotent-transformer` library. It contains the main logic, interfaces, and base classes for creating and managing idempotent operations.

## Core Components

### IdempotentTransformer

The central class of the library. It provides the `makeIdempotent` method to wrap functions and ensure they are executed only once for a given set of inputs within a specific workflow.

### IdempotentFactory

A factory class used to configure and build the `IdempotentTransformer` instance. You must use `IdempotentFactory.build()` to set up the necessary components like storage, serializer, etc., before you can get the transformer instance.

### Base Adapter Classes

The core package provides several base classes that you can extend to create your own custom adapters. This allows you to integrate the `idempotent-transformer` with any storage, serialization, compression, or hashing library you prefer.

- **`IdempotentStateStore`**: Abstract class for implementing a state store. Requires `connect`, `disconnect`, `isConnected`, `save`, and `find` methods.
- **`IdempotentSerializer`**: Abstract class for data serialization. Requires `serialize` and `deserialize` methods. It also has a `configure` method that can be used to set up the serializer.
- **`IdempotentCompressor`**: Abstract class for data compression. Requires `compress`, `decompress`, and `isCompressed` methods.
- **`IdempotentCrypto`**: Abstract class for hashing. Requires a `createHash` method.
- **`IdempotentLogger`**: An interface for logging. You can pass any logger that implements this interface.

## Usage

First, configure the transformer using the `IdempotentFactory`. This should be done once when your application starts.

```typescript
import { IdempotentFactory } from '@idempotent-transformer/core';
// Import your adapter implementations
import { RedisAdapter } from '@idempotent-transformer/adapter-redis';
import { MessagePack } from '@idempotent-transformer/adapter-message-pack';
import { ZstdCompressor } from '@idempotent-transformer/adapter-zstd';
import { Md5Adapter } from '@idempotent-transformer/adapter-crypto';

async function initialize() {
  await IdempotentFactory.build({
    storage: new RedisAdapter({ options: { host: 'localhost', port: 6379 } }),
    serializer: MessagePack.getInstance(),
    compressor: new ZstdCompressor(),
    crypto: new Md5Adapter(),
    log: console, // You can use any logger that implements the IdempotentLogger interface
  });
}
```

After the factory is built, you can get the singleton instance of the `IdempotentTransformer` anywhere in your application.

```typescript
import { IdempotentTransformer } from '@idempotent-transformer/core';

const transformer = IdempotentTransformer.getInstance();

// Now you can use the transformer to make functions idempotent
```

## Creating Custom Adapters

You can create your own adapters by extending the base classes provided by the core package.

Here is an example of a custom logger:

```typescript
import { IdempotentLogger } from '@idempotent-transformer/core';

class MyCustomLogger implements IdempotentLogger {
  debug(message: string, ...args: any[]): void {
    console.debug(`[DEBUG] ${message}`, ...args);
  }
  error(message: string, ...args: any[]): void {
    console.error(`[ERROR] ${message}`, ...args);
  }
  // Implement other methods...
}
```

### The `@Serialize` Decorator

When dealing with classes, especially those with private properties or complex internal state, you need a reliable way to control their serialization and deserialization. The `@Serialize` decorator allows you to establish a clear contract for this process.

By decorating a class with `@Serialize`, you are required to implement two methods: one for serialization (an instance method) and one for deserialization (a static method). This is particularly useful because it allows you to handle private properties and reconstruct the object correctly.

Here's an example of how to use it:

```typescript
import { Serialize } from '@idempotent-transformer/core';

@Serialize({
  name: 'User', // A unique name for this class
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

  // Instance method to serialize the object
  serialize(): { id: string; name: string } {
    return {
      id: this.#id,
      name: this.#name,
    };
  }

  // Static method to deserialize and reconstruct the object
  static deserialize(data: { id: string; name: string }): User {
    return new User(data.id, data.name);
  }

  greet() {
    return `Hello, my name is ${this.#name}`;
  }
}
```

When a serializer that supports this feature (like `@idempotent-transformer/adapter-message-pack`) encounters an instance of `User`, it will use the `serialize` method to get a plain object representation. When deserializing, it will call the static `deserialize` method to create a new `User` instance, ensuring the object is always in a valid state.
