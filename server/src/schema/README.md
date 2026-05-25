# schema/ — Plain data structs

Row types that map directly to RocksDB values (JSON-encoded) or wire values.
No logic, no database calls. All structs derive `Debug + Clone + Serialize + Deserialize`.

## Files

### `workflow.rs`

```rust
pub enum WorkflowStatus { Running = 0, Completed = 1 }

pub struct Workflow {
    pub id: String,
    pub status: i64,          // WorkflowStatus discriminant
    pub expire_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub name: Option<String>,
    pub created_at: i64,      // Unix milliseconds
}
```

Stored as serde_json in `CF_WORKFLOWS` of the state machine RocksDB.
Key: `workflow_id` bytes.

### `checkpoint.rs`

```rust
pub struct CheckpointValue {
    pub position: i64,
    pub idempotency_key: String,
    pub value: Option<Vec<u8>>,
}

pub struct Checkpoint {
    pub workflow_id: String,
    pub position: i64,
    pub value: Option<Vec<u8>>,
    pub idempotency_key: String,
    pub created_at: i64,
}
```

`Checkpoint` is stored as serde_json in `CF_CHECKPOINTS`.
Key: `{workflow_id_bytes}\x00{position.to_be_bytes()}`.

`CheckpointValue` is a view type used in some service response paths.

### `leased_checkpoint.rs`

```rust
pub struct LeasedCheckpointValue {
    pub lease_timeout: i64,  // milliseconds
    pub created_at: i64,     // Unix milliseconds — compute expiry as created_at + lease_timeout
}

pub struct LeasedCheckpoint {  // legacy — not used by current code
    pub workflow_id: String,
    pub position_checksum: i64,
    pub lease_timeout: i64,
    pub created_at: i64,
}
```

`LeasedCheckpointValue` is the value stored in `LeaseRepository`'s moka cache (in-memory only).

### `workflow_fencing_token.rs`

```rust
pub struct WorkflowFencingToken {
    pub workflow_id: String,
    pub fencing_token: i64,
}
```

Legacy struct — `FencingTokenRepository` returns `i64` directly, not this struct.
Fencing tokens are stored as raw 8-byte big-endian `i64` in `CF_FENCING_TOKENS`.

## Dependencies

- `serde::{Serialize, Deserialize}` — JSON encoding for RocksDB storage
