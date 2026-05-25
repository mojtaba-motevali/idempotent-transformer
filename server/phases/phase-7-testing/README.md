# Phase 7 — Testing & Validation

**Status: TODO**

## Goal

Confirm the new openraft + rusqlite implementation is behaviourally identical to the
hiqlite version: same idempotency guarantees, same fencing token protocol, same gRPC API.

## Prerequisites

- `cargo check` clean ✓
- `cargo build` succeeds ✓
- Docker + docker-compose available

## Test plan

### 1. Unit tests — `cargo test`

Use `rusqlite::Connection::open_in_memory()` wrapped in a single-connection pool.
No Raft needed. Focus on correctness of individual methods.

| Target | Scenarios |
|--------|-----------|
| `WorkflowRepository::create_or_get` | Same id → same row (idempotent) |
| `CheckpointRepository::upsert` | Conflict: second upsert updates value |
| `FencingTokenRepository::increment` | Returns 1, 2, 3, … monotonically |
| `LeaseRepository::acquire` | Acquire, get, release cycle; TTL expiry |
| `CheckpointService::checkpoint` | Stale fencing token → abort; valid token → ok |
| `CheckpointService::lease` | Non-determinism detection; remaining timeout |
| `WorkflowService::finish` | Expired token → `WorkflowError::FencingTokenExpired` |

### 2. Integration tests — single node

```bash
cargo test --test '*' -- --nocapture
```

Start a `RaftNode` in test setup (single-node, no peers). Run full RPC flows.

| Scenario | Expected |
|----------|----------|
| `workflow_start` twice, same id | Same id; fencing token increments each call |
| `checkpoint` with stale token | `abort = true` |
| `lease → checkpoint → re-lease` | Second lease returns stored value |
| `generate_idempotency_key` twice, same inputs | Same key (deterministic) |
| `complete_workflow` then expired cleanup | Row deleted from DB after TTL |

### 3. BDD / end-to-end — 3-node cluster

```bash
docker-compose up -d
cargo test --test bdd_tests
```

All existing BDD tests must pass without modification (gRPC API is unchanged).

| Scenario | Verification |
|----------|-------------|
| Normal workflow lifecycle | All steps complete; status = Completed |
| Worker crash mid-lease | Lease expires; next worker succeeds |
| Leader killed mid-workflow | New leader elected; no duplicate execution |
| Concurrent `workflow_start` same id | One winner; others see consistent fencing token |
| Cleanup after expire_after | Row removed from DB |

### 4. Throughput benchmark

```bash
cargo bench --bench checkpoint_throughput
```

N concurrent clients each call `checkpoint` on distinct workflow IDs.
Measure p50/p99 latency and throughput (ops/sec).
Compare against a recorded hiqlite baseline.

### 5. Leader failover test

```bash
docker-compose up -d
# kill the leader container
docker-compose stop node-1
# verify new leader elected; run workflow; verify no data loss
cargo test --test failover
```

Expected: fencing token protocol intact; no duplicate checkpoint written.

## Definition of done

- [ ] `cargo test` — all unit tests pass
- [ ] `cargo test --test '*'` — all integration tests pass
- [ ] BDD suite green on 3-node cluster
- [ ] Throughput benchmark shows improvement over hiqlite baseline
- [ ] Leader failover test passes
- [ ] `grep -r "hiqlite" src/` → no results
- [ ] `grep -rn "println!\|eprintln!" src/` → no results

## Current gaps in overhead_tests/

The existing `overhead_tests/` artillery scripts are **not suitable** as-is:
- No correctness assertions — only measure latency, never verify idempotency
- `parallel-same-workflow.js` uses hardcoded `workflowId = '1000'` with artificial delays
- `experiments/` folder tests an old Redis-based architecture (inapplicable)
- No Raft-specific scenarios (leader failover, split brain)
- Port hardcoded to `51000` (may not match current config)

These scripts can be adapted for throughput benchmarking once port config is fixed,
but correctness testing must be done in Rust integration tests.

## Detail doc

See `testing.md` for full test scenario descriptions and command reference.
