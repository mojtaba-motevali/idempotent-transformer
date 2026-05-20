# Phase 7 — Testing & Validation

## Goal

Confirm that the new openraft + rusqlite implementation is behaviourally identical to the
hiqlite version: same idempotency guarantees, same fencing token protocol, same gRPC API.

---

## Prerequisites

- Phase 5 compiles end-to-end (`cargo build` is green)
- Phase 6 tracing in place (structured logs make failures easier to diagnose)
- Docker and docker-compose available

---

## Test levels

### 1. Unit tests

```bash
cargo test
```

Each repository struct and service method should have unit tests where the logic can be
exercised without a running Raft cluster:

| Target | Test focus |
|--------|-----------|
| `WorkflowRepository` | create_or_get idempotency (same id → same row) |
| `CheckpointRepository` | upsert conflict resolution |
| `FencingTokenRepository` | increment monotonicity |
| `LeaseRepository` | acquire / get / release / TTL expiry |
| `CheckpointService::checkpoint` | fencing token validation, abort flag |
| `CheckpointService::lease` | non-determinism detection, remaining timeout |
| `WorkflowService::finish` | expired token rejection |

Use an in-memory SQLite connection (`rusqlite::Connection::open_in_memory()`) wrapped in a
single-connection pool for unit tests — no need to start openraft.

### 2. Integration tests (single node)

```bash
cargo test --test '*' -- --nocapture
```

Start a single-node `RaftNode` in test setup; run all RPC flows against it.

Scenarios:

| Scenario | Expected |
|----------|----------|
| `workflow_start` twice with same id | Returns same id, incremented fencing token each time |
| `checkpoint` with stale fencing token | `abort = true` |
| `lease_checkpoint` then `checkpoint` then re-`lease_checkpoint` | Second lease returns the value |
| `generate_idempotency_key` twice at same position with same fencing token | Same key (idempotent) |
| `complete_workflow` then `checkpoint` | Fencing token still valid until TTL expires |

### 3. BDD / end-to-end tests (3-node cluster)

```bash
docker-compose up -d
# wait for cluster to elect a leader
cargo test --test bdd_tests
```

All existing BDD tests must pass without modification — the gRPC API is unchanged.

Key scenarios to verify:

| Scenario | Verification |
|----------|-------------|
| Normal workflow lifecycle | All steps complete; status = Completed |
| Worker crash mid-lease | Lease expires; next worker picks up correctly |
| Leader node killed mid-workflow | New leader elected; workflow continues without duplicate |
| Concurrent `workflow_start` for same id | One fencing token wins; others get abort |
| Expired workflow cleanup | After `expire_after` ms, row deleted from DB |

### 4. Concurrent write throughput benchmark

Measures that the new storage backend is actually faster than hiqlite under write load.

```bash
# run the benchmark binary (create if it does not exist)
cargo bench --bench checkpoint_throughput
```

Benchmark: N concurrent clients each call `checkpoint` on distinct workflow IDs.
Measure p50 / p99 latency and total throughput (ops/sec).

Compare against a recorded baseline from the hiqlite version.

### 5. Leader failover test

```bash
# start 3-node cluster
docker-compose up -d

# identify leader (check logs or use a status endpoint)
# kill the leader container
docker-compose stop node-1

# verify cluster recovers and elects a new leader within openraft election timeout
# run a workflow through the new leader
cargo test --test failover
```

Expected: no data loss; no duplicate checkpoint written; fencing token protocol intact.

---

## Tracing-assisted debugging

Run with `RUST_LOG=debug` during tests to confirm structured fields appear correctly:

```bash
RUST_LOG=debug cargo test --test bdd_tests 2>&1 | grep -E "workflow_id|fencing_token|ERROR"
```

---

## Definition of done

- [ ] `cargo test` — all unit tests pass
- [ ] `cargo test --test '*'` — all integration tests pass
- [ ] BDD suite green against 3-node docker-compose cluster
- [ ] Throughput benchmark shows improvement over hiqlite baseline
- [ ] Leader failover test passes
- [ ] `grep -r "hiqlite" src/` returns no results
- [ ] `grep -rn "println!\|eprintln!" src/` returns no results
