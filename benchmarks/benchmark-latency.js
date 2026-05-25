'use strict';

/**
 * 6-step workflow latency benchmark.
 *
 * Measures per-phase latency of a complete workflow:
 *   start → step_1 → step_2 → step_3 → step_4 → step_5 → step_6 → complete
 *
 * Each step calls runner.execute(), which internally calls:
 *   generate_idempotency_key + lease_checkpoint + (task fn) + checkpoint
 *
 * Usage:
 *   node benchmark-latency.js [options]
 *
 * Options:
 *   --workflows  N   Total workflows to measure (default: 200)
 *   --concurrency N  Parallel workflows per batch (default: 1)
 *   --warmup N       Warmup workflows before measuring (default: 10)
 *   --port N         gRPC port (default: 8080)
 */

const { IdempotentTransformer } = require('@idempotent-transformer/core');
const { GrpcAdapter } = require('@idempotent-transformer/grpc-adapter');
const { MessagePack } = require('@idempotent-transformer/message-pack-adapter');

// ── CLI args ──────────────────────────────────────────────────────────────────

const argv = process.argv.slice(2);
function arg(name, def) {
  const i = argv.indexOf(`--${name}`);
  return i !== -1 ? parseInt(argv[i + 1], 10) : def;
}

const N_WORKFLOWS  = arg('workflows',  200);
const CONCURRENCY  = arg('concurrency', 1);
const WARMUP       = arg('warmup',      10);
const PORT         = arg('port',       8080);

// ── Task definitions ──────────────────────────────────────────────────────────
//
// Tasks are instant (no artificial delay) so timings reflect pure engine
// overhead: the gRPC round-trips for lease_checkpoint + checkpoint.

const TASK_PAYLOAD = { result: 'benchmark', value: 1 };
const STEP_NAMES = ['step_1', 'step_2', 'step_3', 'step_4', 'step_5', 'step_6'];
const TASK_FN = async () => TASK_PAYLOAD;

// ── Stats helpers ─────────────────────────────────────────────────────────────

function pct(sorted, p) {
  const idx = Math.ceil((p / 100) * sorted.length) - 1;
  return sorted[Math.max(0, Math.min(idx, sorted.length - 1))];
}

function summarise(samples) {
  if (!samples.length) return null;
  const sorted = [...samples].sort((a, b) => a - b);
  const mean = samples.reduce((s, v) => s + v, 0) / samples.length;
  return {
    n:    samples.length,
    min:  sorted[0],
    mean,
    p50:  pct(sorted, 50),
    p90:  pct(sorted, 90),
    p95:  pct(sorted, 95),
    p99:  pct(sorted, 99),
    max:  sorted[sorted.length - 1],
  };
}

function fmt(ms) {
  if (ms == null) return '—';
  return ms.toFixed(1) + 'ms';
}

function printTable(rows) {
  const COLS = ['phase', 'n', 'min', 'mean', 'p50', 'p90', 'p95', 'p99', 'max'];
  const widths = COLS.map(c => Math.max(c.length, ...rows.map(r => String(r[c] ?? '').length)));

  const sep = '+' + widths.map(w => '-'.repeat(w + 2)).join('+') + '+';
  const row = r => '|' + COLS.map((c, i) => {
    const v = String(r[c] ?? '');
    // right-align numeric columns
    return c === 'phase' ? ' ' + v.padEnd(widths[i]) + ' ' : ' ' + v.padStart(widths[i]) + ' ';
  }).join('|') + '|';

  console.log(sep);
  console.log(row(Object.fromEntries(COLS.map(c => [c, c]))));
  console.log(sep);
  for (const r of rows) console.log(row(r));
  console.log(sep);
}

// ── Single workflow run ───────────────────────────────────────────────────────

async function runOne(transformer, id, record) {
  const t0 = performance.now();

  const runner = await transformer.startWorkflow(id, { completedRetentionTime: 120_000 });
  const afterStart = performance.now();
  if (record) record.start.push(afterStart - t0);

  for (const name of STEP_NAMES) {
    const ts = performance.now();
    await runner.execute(name, TASK_FN);
    if (record) record[name].push(performance.now() - ts);
  }

  const tc = performance.now();
  await runner.complete();
  if (record) record.complete.push(performance.now() - tc);

  if (record) record.total.push(performance.now() - t0);
}

// ── Batch runner ──────────────────────────────────────────────────────────────

async function runBatch(transformer, ids, record) {
  for (let i = 0; i < ids.length; i += CONCURRENCY) {
    const slice = ids.slice(i, i + CONCURRENCY);
    await Promise.all(slice.map(id => runOne(transformer, id, record)));
    if (record) {
      process.stdout.write(`\r  measured ${record.total.length}/${N_WORKFLOWS}`);
    } else {
      process.stdout.write(`\r  warmed up ${i + slice.length}/${WARMUP}`);
    }
  }
  process.stdout.write('\n');
}

// ── Main ──────────────────────────────────────────────────────────────────────

async function main() {
  const transformer = new IdempotentTransformer({
    rpcAdapter: new GrpcAdapter({ host: 'localhost', port: PORT }),
    serializer: MessagePack.getInstance(),
    log: null,
  });

  const PHASE_NAMES = ['start', ...STEP_NAMES, 'complete', 'total'];
  const timings = Object.fromEntries(PHASE_NAMES.map(n => [n, []]));

  console.log('');
  console.log(`6-step workflow latency benchmark`);
  console.log(`  server   : localhost:${PORT}`);
  console.log(`  warmup   : ${WARMUP} workflows`);
  console.log(`  measure  : ${N_WORKFLOWS} workflows`);
  console.log(`  concurr  : ${CONCURRENCY} parallel`);
  console.log('');

  // Warmup — don't record
  const runId = Date.now();
  if (WARMUP > 0) {
    console.log('Warming up...');
    const warmIds = Array.from({ length: WARMUP }, (_, i) => `bench-warmup-${runId}-${i}`);
    await runBatch(transformer, warmIds, null);
  }

  // Measure
  console.log('Measuring...');
  const measureIds = Array.from({ length: N_WORKFLOWS }, (_, i) => `bench-latency-${runId}-${i}`);
  const wall0 = performance.now();
  await runBatch(transformer, measureIds, timings);
  const wallMs = performance.now() - wall0;

  // Results
  console.log('');
  const rows = PHASE_NAMES.map(name => {
    const s = summarise(timings[name]);
    if (!s) return { phase: name };
    return {
      phase:   name,
      n:       s.n,
      min:     fmt(s.min),
      mean:    fmt(s.mean),
      p50:     fmt(s.p50),
      p90:     fmt(s.p90),
      p95:     fmt(s.p95),
      p99:     fmt(s.p99),
      max:     fmt(s.max),
    };
  });

  printTable(rows);

  const wps = (N_WORKFLOWS / (wallMs / 1000)).toFixed(1);
  console.log(`  ${N_WORKFLOWS} workflows in ${(wallMs / 1000).toFixed(2)}s  →  ${wps} workflows/sec`);
  console.log('');
}

main().catch(err => {
  console.error('\nError:', err.message ?? err);
  process.exit(1);
});
