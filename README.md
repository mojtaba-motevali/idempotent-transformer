# Idempotent Transformer

A TypeScript library for creating and working with idempotent transformations.

## Installation

```bash
npm install idempotent-transformer
```

## Usage

```typescript
import { createTransformer } from 'idempotent-transformer';

// Create a transformer that converts strings to lowercase
const lowercaseTransformer = createTransformer((input: string) => input.toLowerCase());

// Use the transformer
const result = lowercaseTransformer.transform('HELLO WORLD');
console.log(result); // 'hello world'

// The transformation is idempotent
const result2 = lowercaseTransformer.transform(result);
console.log(result2); // 'hello world' (same as result)
```

## Features

- Type-safe transformations
- Generic interface for any data type
- Simple and intuitive API
- Fully tested
- Written in TypeScript

## Development

```bash
# Install dependencies
npm install

# Run tests
npm test

# Build the library
npm run build

# Lint the code
npm run lint

# Format the code
npm run format
```

## License

MIT
