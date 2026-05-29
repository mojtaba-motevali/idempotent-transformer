use std::ops::Bound;

use fjall::{Config, Keyspace, PartitionHandle};
use openraft::{
    storage::{RaftLogReader, RaftLogStorage},
    Entry, LogId, LogState, OptionalSend, RaftLogId, StorageError, StorageIOError, Vote,
};

use crate::storage::type_config::{NodeId, TypeConfig};

pub struct LogStore {
    #[allow(dead_code)]
    keyspace: Keyspace,
    logs: PartitionHandle,
    meta: PartitionHandle,
}

pub struct LogStoreReader {
    logs: PartitionHandle,
}

impl LogStore {
    pub fn open(path: &str) -> Result<Self, fjall::Error> {
        let keyspace = Config::new(path)
            // Background fsync every 1 s — bounded durability without blocking writes.
            // Per-operation fsync is ~1-10 ms and kills throughput; this matches
            // the original RocksDB behaviour (no explicit fsync per write).
            .fsync_ms(Some(1_000))
            .open()?;
        let logs = keyspace.open_partition("logs", Default::default())?;
        let meta = keyspace.open_partition("meta", Default::default())?;
        Ok(Self { keyspace, logs, meta })
    }
}

fn log_key(index: u64) -> [u8; 8] {
    index.to_be_bytes()
}

fn map_err_read(e: fjall::Error) -> StorageError<NodeId> {
    StorageError::IO {
        source: StorageIOError::read_logs(&e),
    }
}

fn map_err_write(e: fjall::Error) -> StorageError<NodeId> {
    StorageError::IO {
        source: StorageIOError::write_logs(&e),
    }
}

fn read_range(
    logs: &PartitionHandle,
    start: u64,
    end: u64,
) -> Result<Vec<Entry<TypeConfig>>, StorageError<NodeId>> {
    let mut entries = Vec::new();
    for item in logs.range(log_key(start)..log_key(end)) {
        let (_k, v) = item.map_err(map_err_read)?;
        let entry: Entry<TypeConfig> =
            serde_json::from_slice(&v).map_err(|e| StorageError::IO {
                source: StorageIOError::read_logs(&e),
            })?;
        entries.push(entry);
    }
    Ok(entries)
}

impl RaftLogReader<TypeConfig> for LogStoreReader {
    async fn try_get_log_entries<RB>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry<TypeConfig>>, StorageError<NodeId>>
    where
        RB: std::ops::RangeBounds<u64> + Clone + std::fmt::Debug + OptionalSend,
    {
        let start = match range.start_bound() {
            Bound::Included(&i) => i,
            Bound::Excluded(&i) => i + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&i) => i + 1,
            Bound::Excluded(&i) => i,
            Bound::Unbounded => u64::MAX,
        };
        read_range(&self.logs, start, end)
    }
}

impl RaftLogReader<TypeConfig> for LogStore {
    async fn try_get_log_entries<RB>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry<TypeConfig>>, StorageError<NodeId>>
    where
        RB: std::ops::RangeBounds<u64> + Clone + std::fmt::Debug + OptionalSend,
    {
        let start = match range.start_bound() {
            Bound::Included(&i) => i,
            Bound::Excluded(&i) => i + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&i) => i + 1,
            Bound::Excluded(&i) => i,
            Bound::Unbounded => u64::MAX,
        };
        read_range(&self.logs, start, end)
    }
}

impl RaftLogStorage<TypeConfig> for LogStore {
    type LogReader = LogStoreReader;

    async fn get_log_state(&mut self) -> Result<LogState<TypeConfig>, StorageError<NodeId>> {
        let last = self
            .logs
            .range(log_key(0)..=log_key(u64::MAX))
            .next_back()
            .transpose()
            .map_err(map_err_read)?
            .and_then(|(_k, v)| {
                serde_json::from_slice::<Entry<TypeConfig>>(&v)
                    .ok()
                    .map(|e| *e.get_log_id())
            });

        let purged = match self.meta.get(b"purged").map_err(map_err_read)? {
            Some(bytes) => serde_json::from_slice::<LogId<NodeId>>(&bytes).ok(),
            None => None,
        };

        Ok(LogState {
            last_purged_log_id: purged,
            last_log_id: last,
        })
    }

    async fn save_committed(
        &mut self,
        committed: Option<LogId<NodeId>>,
    ) -> Result<(), StorageError<NodeId>> {
        let bytes = serde_json::to_vec(&committed).map_err(|e| StorageError::IO {
            source: StorageIOError::write_logs(&e),
        })?;
        self.meta.insert(b"committed", bytes).map_err(map_err_write)
    }

    async fn read_committed(&mut self) -> Result<Option<LogId<NodeId>>, StorageError<NodeId>> {
        match self.meta.get(b"committed").map_err(map_err_read)? {
            Some(bytes) => {
                let v = serde_json::from_slice(&bytes).map_err(|e| StorageError::IO {
                    source: StorageIOError::read_logs(&e),
                })?;
                Ok(v)
            }
            None => Ok(None),
        }
    }

    async fn save_vote(&mut self, vote: &Vote<NodeId>) -> Result<(), StorageError<NodeId>> {
        let bytes = serde_json::to_vec(vote).map_err(|e| StorageError::IO {
            source: StorageIOError::write_vote(&e),
        })?;
        self.meta.insert(b"vote", bytes).map_err(|e| StorageError::IO {
            source: StorageIOError::write_vote(&e),
        })
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<NodeId>>, StorageError<NodeId>> {
        match self.meta.get(b"vote").map_err(|e| StorageError::IO {
            source: StorageIOError::read_vote(&e),
        })? {
            Some(bytes) => {
                let vote: Vote<NodeId> = serde_json::from_slice(&bytes).map_err(|e| {
                    StorageError::IO {
                        source: StorageIOError::read_vote(&e),
                    }
                })?;
                Ok(Some(vote))
            }
            None => Ok(None),
        }
    }

    async fn append<I>(
        &mut self,
        entries: I,
        callback: openraft::storage::LogFlushed<TypeConfig>,
    ) -> Result<(), StorageError<NodeId>>
    where
        I: IntoIterator<Item = Entry<TypeConfig>> + OptionalSend,
    {
        for entry in entries {
            let lid = *entry.get_log_id();
            let bytes = serde_json::to_vec(&entry).map_err(|e| StorageError::IO {
                source: StorageIOError::write_logs(&e),
            })?;
            self.logs.insert(log_key(lid.index), bytes).map_err(map_err_write)?;
        }
        callback.log_io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        let state = self.get_log_state().await?;
        let last_index = state.last_log_id.map(|l| l.index).unwrap_or(0);

        for idx in log_id.index..=last_index {
            self.logs.remove(log_key(idx)).map_err(map_err_write)?;
        }
        Ok(())
    }

    async fn purge(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        // Determine the start of the range we actually need to delete.
        // Previous purges already removed [0, last_purged], so only delete
        // (last_purged, log_id.index].
        let from = match self.meta.get(b"purged").map_err(map_err_read)? {
            Some(bytes) => serde_json::from_slice::<LogId<NodeId>>(&bytes)
                .map(|l| l.index + 1)
                .unwrap_or(0),
            None => 0,
        };

        // Update the marker immediately so get_log_state() returns the correct
        // last_purged_log_id without waiting for the deletions to finish.
        let bytes = serde_json::to_vec(&log_id).map_err(|e| StorageError::IO {
            source: StorageIOError::write_logs(&e),
        })?;
        self.meta.insert(b"purged", bytes).map_err(map_err_write)?;

        // Offload the actual entry deletions so the Raft core task is not
        // blocked.  openraft never requests entries with index <= purged, so
        // they can be deleted asynchronously without any correctness risk.
        let logs = self.logs.clone();
        tokio::spawn(async move {
            for idx in from..=log_id.index {
                let _ = logs.remove(log_key(idx));
            }
        });

        Ok(())
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        LogStoreReader {
            logs: self.logs.clone(),
        }
    }
}
