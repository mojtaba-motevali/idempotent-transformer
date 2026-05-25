# Prerequisites

- Node.js 20
- Docker Or Rust

# Run the server

## Clustered (Docker)

```bash
cd ../server
docker compose up -d
```

## Single node (local Rust binary)

```bash
cd ../server
cargo build --release
./target/release/idempotency-server \
  --node-id 1 \
  --addr-raft 127.0.0.1:8200 \
  --rpc-addr 0.0.0.0:8080 \
  --data-dir ./data/node_1
```

# Run the tests

To run stress tests against a single node (port 8080):

```bash
npm run test:parallel-same
```

To run stress tests against a cluster (port 51000, node1 direct):

```bash
npm run test:parallel-diff
```

To run high-availability stress tests:

```bash
npm run test:parallel-ha
```

# To turn off the artillery's warning

run:

```bash
export NODE_NO_WARNINGS=1
```
