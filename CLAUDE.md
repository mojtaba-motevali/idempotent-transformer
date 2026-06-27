# SyncShield — Project Overview

A distributed idempotency engine for multi-step workflows. Callers execute workflow steps exactly once, even under retries and concurrent workers.

## Repository layout

```
server/              Rust gRPC server (openraft + fjall + tonic) — see server/CLAUDE.md
packages/
  core/              @idempotent-transformer/core — framework-agnostic TS client
  grpc-adapter/      @idempotent-transformer/grpc-adapter — gRPC transport layer
  message-pack-adapter/ @idempotent-transformer/message-pack-adapter — msgpack serializer
  nestjs/            @idempotent-transformer/nestjs — NestJS integration module
bdd_tests/           Cucumber BDD tests (against a running server cluster)
benchmarks/          Artillery + custom JS load tests
examples/
  nestjs/            NestJS example app
  nextjs/            Next.js example app
```

## TypeScript workspace (pnpm)

```bash
pnpm install          # install all packages
pnpm -r run build     # build all packages
pnpm test             # run Jest unit tests (root)
```

Packages are in a pnpm workspace (`pnpm-workspace.yaml`). Each package has its own `tsconfig.json` and `package.json`.

## BDD tests

Require a running 3-node cluster (see `server/docker-compose.yaml`):

```bash
cd bdd_tests && pnpm test
```

## Key conventions

- TypeScript: strict mode, ESLint + Prettier (`.eslintrc.json`, `.prettierrc`)
- Server: Rust 2024 edition — see `server/CLAUDE.md` for all server details
