# Benchmarks

## Prerequisites

- Node.js 20
- Docker **or** a local Rust build

## Start the server

### Single node (local Rust binary)

```bash
cd ../server
cargo build --release
./target/release/idempotency-server \
  --node-id 1 \
  --addr-raft 127.0.0.1:8200 \
  --rpc-addr 0.0.0.0:8080 \
  --data-dir ./data/node_1
```

### Clustered (Docker)

```bash
cd ../server
docker compose up -d
```

---

## Latency benchmark — `benchmark-latency.js`

Runs complete 6-step workflows and prints a per-phase latency table.
This is the primary tool for understanding how long each part of a workflow takes.

### What is measured

```
workflow_start → step_1 → step_2 → step_3 → step_4 → step_5 → step_6 → complete
     ↑             ↑                                                         ↑
  1 gRPC      each step:                                                 end-to-end
               generate_idempotency_key  (1 gRPC)
               + lease_checkpoint        (1 gRPC)
               + task fn (instant here)
               + checkpoint              (1 gRPC)
```

Tasks in this benchmark return a payload immediately — no artificial delay — so timings
reflect pure engine overhead (Raft consensus + RocksDB writes + gRPC round-trips).

### Run

```bash
# Default: 10 warmup + 200 measured workflows, concurrency=1, port=8080
npm run test:latency

# Custom options
node benchmark-latency.js --workflows 500 --concurrency 4 --warmup 20 --port 8080

# Quick CI run
npm run test:latency:ci
```

### Example output

```
6-step workflow latency benchmark
  server   : localhost:8080
  warmup   : 10 workflows
  measure  : 200 workflows
  concurr  : 1 parallel

Warming up...
  warmed up 10/10
Measuring...
  measured 200/200

+-----------+-----+---------+---------+---------+---------+---------+---------+---------+
| phase     |   n |     min |    mean |     p50 |     p90 |     p95 |     p99 |     max |
+-----------+-----+---------+---------+---------+---------+---------+---------+---------+
| start     | 200 |   1.2ms |   2.1ms |   1.9ms |   3.4ms |   4.2ms |   7.1ms |  15.3ms |
| step_1    | 200 |   3.5ms |   5.8ms |   5.4ms |   8.6ms |  10.1ms |  16.4ms |  28.2ms |
| step_2    | 200 |   3.3ms |   5.6ms |   5.2ms |   8.4ms |   9.8ms |  15.9ms |  26.7ms |
| step_3    | 200 |   3.4ms |   5.7ms |   5.3ms |   8.5ms |   9.9ms |  16.1ms |  27.4ms |
| step_4    | 200 |   3.2ms |   5.5ms |   5.1ms |   8.3ms |   9.7ms |  15.7ms |  26.1ms |
| step_5    | 200 |   3.5ms |   5.8ms |   5.4ms |   8.6ms |  10.2ms |  16.5ms |  28.5ms |
| step_6    | 200 |   3.3ms |   5.6ms |   5.2ms |   8.4ms |   9.8ms |  15.9ms |  26.7ms |
| complete  | 200 |   1.1ms |   1.9ms |   1.7ms |   3.0ms |   3.7ms |   6.5ms |  13.8ms |
| total     | 200 |  26.4ms |  43.8ms |  41.2ms |  63.5ms |  73.4ms | 104.2ms | 178.6ms |
+-----------+-----+---------+---------+---------+---------+---------+---------+---------+

  200 workflows in 8.77s  →  22.8 workflows/sec
```

### CLI flags

| Flag | Default | Description |
|------|---------|-------------|
| `--workflows N` | `200` | Total workflows to measure |
| `--concurrency N` | `1` | Parallel workflows per batch. Use `1` for pure latency; increase to see latency under load |
| `--warmup N` | `10` | Workflows run before measuring (warms up gRPC connection + Raft leader) |
| `--port N` | `8080` | gRPC port of the server |

---

## Artillery benchmarks

Artillery drives many virtual users simultaneously to test throughput and latency under load.

`parallel-different-workflow.js` emits Artillery histogram metrics for each workflow phase:
`workflow.start_ms`, `workflow.task1_ms` … `workflow.task6_ms`, `workflow.complete_ms`,
`workflow.total_ms`. These appear in the Artillery JSON report as p50/p90/p99 histograms.

### Latency at low concurrency

5 arrivals/second for 60 seconds (~300 samples). Produces the same latency picture as
`benchmark-latency.js` but within Artillery's reporting format.

```bash
npm run test:latency-artillery
# output → report-latency.json
npx artillery report report-latency.json
```

### Throughput — different workflow IDs

Ramps from 1 to 1500 concurrent virtual users over 120 seconds.
Each virtual user runs a unique 6-step workflow (measures server capacity).

```bash
npm run test:parallel-diff
# output → report-diff.json
```

### Throughput — same workflow ID (idempotency replay)

5 arrivals/second for 10 seconds, all against `workflowId = '1000'`.
Tests that concurrent replays are idempotent (first runner executes each step, others skip).

```bash
npm run test:parallel-same
# output → report-same.json
```

### High availability

Ramps from 1 to 300 concurrent users over 120 seconds against the 3-node cluster.

```bash
npm run test:parallel-ha
# output → report-high-available.json
```

---

## Suppress Artillery warnings

```bash
export NODE_NO_WARNINGS=1
```
