# Phase 2 — LogStore

## File

`src/storage/log_store.rs`

## Purpose

Implements `openraft::storage::RaftLogStore<TypeConfig>` on top of RocksDB.
The Raft log is append-only and write-heavy — RocksDB handles concurrent column-family
writes without a single-writer bottleneck.

---

## Struct

```rust
use std::sync::Arc;
use rocksdb::{ColumnFamilyDescriptor, Options, DB};

pub struct LogStore {
    db: Arc<DB>,
}

impl LogStore {
    /// Opens (or creates) the RocksDB instance at `path`.
    /// Two column families: "logs" and "meta".
    pub fn open(path: &str) -> Result<Self, rocksdb::Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_logs = ColumnFamilyDescriptor::new("logs", Options::default());
        let cf_meta = ColumnFamilyDescriptor::new("meta", Options::default());

        let db = DB::open_cf_descriptors(&opts, path, vec![cf_logs, cf_meta])?;
        Ok(Self { db: Arc::new(db) })
    }
}
```

---

## RaftLogStore implementation

openraft requires these methods. Each maps to a RocksDB operation:

| openraft method | RocksDB operation | CF |
|----------------|------------------|----|
| `append(entries)` | `db.put_cf(logs, log_id_key, bincode/serde_json entry)` | `logs` |
| `truncate(since)` | delete range `[since, last]` in `logs` CF | `logs` |
| `purge(upto)` | delete range `[0, upto]` in `logs` CF | `logs` |
| `get_log_reader()` | returns `LogStoreReader` wrapping `Arc<DB>` | `logs` |
| `save_vote(vote)` | `db.put_cf(meta, "vote", serialized)` | `meta` |
| `read_vote()` | `db.get_cf(meta, "vote")` | `meta` |

Key encoding for log entries:

```rust
// big-endian u64 so lexicographic order == log order
fn log_key(index: u64) -> [u8; 8] {
    index.to_be_bytes()
}
```

---

## Notes

- Use `serde_json` to serialize `openraft::Entry<TypeConfig>` (already a serde dependency).
- `LogStoreReader` must also implement `openraft::storage::RaftLogReader<TypeConfig>`.
- RocksDB `Arc<DB>` is `Send + Sync`; clone the `Arc` for the reader.
- Column family handles must be retrieved per operation via `db.cf_handle("logs").unwrap()`.

---

## Verification

```bash
cargo check -p idempotency-server
# LogStore must implement RaftLogStore<TypeConfig> with no compiler errors
```
