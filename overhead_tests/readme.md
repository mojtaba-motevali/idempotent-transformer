# Prerequisites

- Node.js 20
- Docker Or Rust

# Run the server

```bash
docker compose up -d
```

# Run the tests

To run stress tests for the server, you can use the following command:

```bash
npm run test:parallel-different-workflow
```

To run the parallelism safety test:

```bash
npm run test:parallel-same
```

# To turn off the artillery's warning

run:

```bash
export NODE_NO_WARNINGS=1

```
