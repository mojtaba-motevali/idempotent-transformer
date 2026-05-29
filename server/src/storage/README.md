# storage/ — openraft storage and network implementation

Implements the three openraft traits: `RaftLogStorage`, `RaftStateMachine`, and
`RaftNetwork` / `RaftNetworkFactory`. Also defines the Raft type parameters and
the command/response enums that travel through the log.

## Files

### `type_config.rs`

```rust
pub type NodeId = u64;

pub struct Node { pub addr_raft: String }

declare_raft_types!(
    pub TypeConfig:
        D            = WorkflowCommand,
        R            = WorkflowResponse,
        NodeId       = NodeId,
        Node         = Node,
        Entry        = openraft::Entry<TypeConfig>,
        SnapshotData = std::io::Cursor<Vec<u8>>,
);
```

`TypeConfig` is the single type parameter passed to every openraft generic.

### `command.rs`

```rust
pub enum WorkflowCommand {
    CreateOrGetWorkflow { workflow_id, name, created_at },
    IncrementFencingToken { workflow_id },
    UpsertCheckpoint { workflow_id, position, idempotency_key, value: Option<Vec<u8>>, created_at },
    CompleteWorkflow { workflow_id, expire_at, completed_at },
    DeleteExpired { before_ms },
}

pub enum WorkflowResponse {
    Workflow { id, fencing_token },  // unused by current apply dispatch
    FencingToken(i64),               // returned by IncrementFencingToken
    Empty,                           // returned by all other commands
}
```

Both enums are `Serialize + Deserialize` — serialised into the fjall log via `serde_json`.

### `log_store.rs` — `LogStore`

```rust
pub struct LogStore {
    keyspace: Keyspace,
    logs: PartitionHandle,
    meta: PartitionHandle,
}
pub struct LogStoreReader { logs: PartitionHandle }
```

Implements `RaftLogStorage<TypeConfig>` + `RaftLogReader<TypeConfig>`.

Uses fjall (`fjall::Keyspace`) for durability. `LogStore::open(path) -> Result<Self, fjall::Error>` opens (or creates) the keyspace with a 1-second background fsync interval.

**Layout** (keyspace at `{data_dir}/raft-log`):
- Partition `logs`: key = `u64` index as 8-byte big-endian, value = serde_json entry
- Partition `meta`: keys `vote`, `committed`, `purged` → serde_json values

### `state_machine.rs` — `StateMachine`

All application data lives in **RAM** inside a `SharedStore`:

```rust
pub struct Store {
    pub(crate) workflows:      HashMap<String, Workflow>,
    pub(crate) fencing_tokens: HashMap<String, i64>,
    pub(crate) checkpoints:    BTreeMap<(String, i64), Checkpoint>,
}

pub type SharedStore = Arc<parking_lot::Mutex<Store>>;
```

`StateMachine::new(data_dir: &str) -> anyhow::Result<(StateMachine, SharedStore)>`:
- Tries to restore from `{data_dir}/snapshot.bin` (self-contained JSON file)
- Returns `(StateMachine, SharedStore)` — the same `SharedStore` is handed to repositories

```rust
pub struct StateMachine {
    store:           SharedStore,
    last_applied:    parking_lot::Mutex<Option<LogId<u64>>>,
    last_membership: parking_lot::Mutex<StoredMembership<u64, Node>>,
    last_snapshot:   Arc<parking_lot::Mutex<Option<(SnapshotMeta<u64, Node>, Vec<u8>)>>>,
    snapshot_path:   PathBuf,
}
```

**Mutex choice rule**: `parking_lot::Mutex` everywhere — all critical sections are inline
in-memory operations with no `.await` while the lock is held.

**`apply(entries)`** — per entry:
- `Normal(cmd)` → `store.lock()` → `apply_command(&mut store, cmd)` → drop lock
- `Membership(m)` → `last_membership.lock() = ...` (no store access)
- Updates `last_applied.lock()` after each entry

**`apply_command` dispatch (free function, synchronous):**
```
CreateOrGetWorkflow   → HashMap::entry → insert or update name    → Empty
IncrementFencingToken → HashMap::entry → and_modify(+1)/or_insert(1) → FencingToken(n)
UpsertCheckpoint      → BTreeMap::insert                           → Empty
CompleteWorkflow      → HashMap::get_mut → set status/expire_at    → Empty
DeleteExpired         → collect expired IDs → remove all three maps → Empty
```

**Snapshot** — `StateSnapshotBuilder`:

```rust
pub struct StateSnapshotBuilder {
    // Point-in-time copy captured at get_snapshot_builder() time
    snapshot_workflows:      Vec<Workflow>,
    snapshot_fencing_tokens: Vec<(String, i64)>,
    snapshot_checkpoints:    Vec<Checkpoint>,
    last_applied:            Option<LogId<u64>>,
    last_membership:         StoredMembership<u64, Node>,
    last_snapshot: Arc<parking_lot::Mutex<...>>,
    snapshot_path: PathBuf,
}
```

`get_snapshot_builder()`: locks store briefly to clone all data, then releases before
reading `last_applied`/`last_membership`.

`build_snapshot()`: serializes the pre-cloned data to serde_json, writes
`snapshot_path`, updates `last_snapshot` cache. After this returns, openraft calls
`LogStore::purge(meta.last_log_id)` to **vacuum committed log entries**.

`install_snapshot()`: parses JSON → locks store → clears+fills maps → unlocks →
updates metadata locks → persists to `snapshot_path`.

`get_current_snapshot()`: returns from `last_snapshot` cache.

### `network.rs` — `NetworkFactory` + `Network` + `PeerConn`

```rust
pub struct NetworkFactory { pool: Arc<DashMap<NodeId, Arc<PeerConn>>> }
pub struct Network { conn: Arc<PeerConn> }
struct PeerConn {
    addr:     String,
    state:    RwLock<ConnState>,   // write_tx: Option<UnboundedSender>
    pending:  Arc<DashMap<u64, oneshot::Sender<Result<Vec<u8>, NetworkError>>>>,
    next_id:  AtomicU64,
}
```

`NetworkFactory` implements `RaftNetworkFactory<TypeConfig>`:
- `new_client(target, node)` → `Network` backed by the shared `PeerConn` for that node (one per remote node)

`Network` implements `RaftNetwork<TypeConfig>`:
- `append_entries`, `vote`, `install_snapshot` — encode request via `network_codec`, send via
  `PeerConn::send()`, decode response

`PeerConn::send()`:
- Lazy connect on first use; spawns a background reader task + writer task per connection
- `RwLock<ConnState>` is held only during (re)connection — never across the response wait
- Concurrent sends proceed in parallel: each registers a oneshot waiter, releases the lock,
  sends its frame to the writer channel, then awaits its own oneshot independently
- TCP_NODELAY enabled (prevents Nagle delay on Raft heartbeats)

### `network_codec.rs`

Wire format: **1-byte type discriminant + serde_json body**.

```
Byte 0:  message type (1–6)
Bytes 1+: serde_json encoded body
```

Discriminants:
```
MSG_APPEND_ENTRIES_REQ  = 1    MSG_APPEND_ENTRIES_RESP  = 2
MSG_VOTE_REQ            = 3    MSG_VOTE_RESP            = 4
MSG_INSTALL_SNAPSHOT_REQ = 5   MSG_INSTALL_SNAPSHOT_RESP = 6
```

Encode functions (client): `encode_append_entries_request`, `encode_vote_request`, `encode_install_snapshot_request`
Decode functions (client): `decode_append_entries_response`, `decode_vote_response`, `decode_install_snapshot_response`
Encode/decode functions (server): symmetric counterparts

Note: FlatBuffers schema at `flatbuffers/raft_network.fbs` exists but is optional.
JSON codec is the production implementation.

### `framing.rs`

Length-prefixed TCP framing: **4-byte big-endian payload-len + 8-byte big-endian req_id + payload** (12-byte header).

```rust
pub async fn write_framed<W: AsyncWriteExt + Unpin>(w: &mut W, req_id: u64, data: &[u8]) -> Result<(), NetworkError>
pub async fn read_framed<R: AsyncReadExt + Unpin>(r: &mut R) -> Result<(u64, Vec<u8>), NetworkError>
```

`req_id` is echoed verbatim in the response so the background reader task can route each
response to the correct `oneshot::Sender`.

### `tcp_server.rs` — `TcpRaftServer`

```rust
pub struct TcpRaftServer { pub raft: Arc<Raft<TypeConfig>> }
impl TcpRaftServer {
    pub fn new(raft: Arc<Raft<TypeConfig>>) -> Arc<Self>
    pub async fn listen(self: Arc<Self>, addr: &str) -> Result<(), NetworkError>
}
```

`listen()` accepts connections in a loop, spawning a task per connection.
Each task runs `handle_conn()`: reads frames, dispatches to `dispatch()`, writes response
with the same `req_id` so the client's reader task can route it.
`dispatch()` peeks the discriminant byte, routes to the local raft instance.

## Persistence model

| Component | Storage | Path |
|-----------|---------|------|
| Raft log | fjall keyspace | `{data_dir}/raft-log` |
| Application state | RAM (HashMap/BTreeMap) | — |
| Snapshot | JSON file | `{data_dir}/state/snapshot.bin` |

The state machine has no RocksDB dependency. On restart, state is restored from the
snapshot file; openraft then replays any log entries committed after the snapshot.

## Dependencies

- `openraft 0.9` — `RaftLogStorage`, `RaftStateMachine`, `RaftNetworkFactory`, `RaftNetwork`
- `fjall` — log store persistence
- `serde_json` — log entry, metadata, and snapshot serialisation
- `parking_lot` — sync mutexes for state machine (no await while held)
- `dashmap` — lock-free peer connection pool + pending-response map
- `tokio::sync::{mpsc, oneshot}` — async writer channel + per-request response routing
- `error::NetworkError`
