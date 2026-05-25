use std::ops::Bound;
use std::sync::Arc;

use openraft::{
    storage::{RaftLogReader, RaftLogStorage},
    Entry, LogId, LogState, OptionalSend, RaftLogId, StorageError, Vote,
};
use rocksdb::{ColumnFamilyDescriptor, Options, DB};

use crate::storage::type_config::{NodeId, TypeConfig};

pub struct LogStore {
    db: Arc<DB>,
}

pub struct LogStoreReader {
    db: Arc<DB>,
}

impl LogStore {
    pub fn open(path: &str) -> Result<Self, rocksdb::Error> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_logs = ColumnFamilyDescriptor::new("logs", Options::default());
        let cf_meta = ColumnFamilyDescriptor::new("meta", Options::default());

        let db = DB::open_cf_descriptors(&opts, path, vec![cf_logs, cf_meta])?;
        Ok(Self { db: Arc::new(db) })
    }

    fn logs_cf(&self) -> &rocksdb::ColumnFamily {
        self.db.cf_handle("logs").unwrap()
    }

    fn meta_cf(&self) -> &rocksdb::ColumnFamily {
        self.db.cf_handle("meta").unwrap()
    }
}

fn log_key(index: u64) -> [u8; 8] {
    index.to_be_bytes()
}

fn read_range(db: &DB, start: u64, end: u64) -> Result<Vec<Entry<TypeConfig>>, StorageError<NodeId>> {
    let cf = db.cf_handle("logs").unwrap();
    let mut entries = Vec::new();
    let iter = db.iterator_cf(
        cf,
        rocksdb::IteratorMode::From(&log_key(start), rocksdb::Direction::Forward),
    );
    for item in iter {
        let (k, v) = item.map_err(|e| StorageError::IO {
            source: openraft::StorageIOError::read_logs(&e),
        })?;
        let idx = u64::from_be_bytes(k.as_ref().try_into().unwrap_or([0u8; 8]));
        if idx >= end {
            break;
        }
        let entry: Entry<TypeConfig> = serde_json::from_slice(&v).map_err(|e| {
            StorageError::IO {
                source: openraft::StorageIOError::read_logs(&e),
            }
        })?;
        entries.push(entry);
    }
    Ok(entries)
}

// RaftLogReader is required by RaftLogStorage as a supertrait, so implement on both types.

impl RaftLogReader<TypeConfig> for LogStoreReader {
    async fn try_get_log_entries<RB: std::ops::RangeBounds<u64> + Clone + std::fmt::Debug + OptionalSend>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry<TypeConfig>>, StorageError<NodeId>> {
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
        read_range(&self.db, start, end)
    }
}

impl RaftLogReader<TypeConfig> for LogStore {
    async fn try_get_log_entries<RB: std::ops::RangeBounds<u64> + Clone + std::fmt::Debug + OptionalSend>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry<TypeConfig>>, StorageError<NodeId>> {
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
        read_range(&self.db, start, end)
    }
}

impl RaftLogStorage<TypeConfig> for LogStore {
    type LogReader = LogStoreReader;

    async fn get_log_state(&mut self) -> Result<LogState<TypeConfig>, StorageError<NodeId>> {
        let cf = self.logs_cf();

        let last = {
            let mut iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::End);
            match iter.next() {
                Some(Ok((_k, v))) => {
                    let entry: Entry<TypeConfig> = serde_json::from_slice(&v).map_err(|e| {
                        StorageError::IO {
                            source: openraft::StorageIOError::read_logs(&e),
                        }
                    })?;
                    Some(*entry.get_log_id())
                }
                _ => None,
            }
        };

        let purged = {
            let meta_cf = self.meta_cf();
            match self.db.get_cf(meta_cf, b"purged").map_err(|e| StorageError::IO {
                source: openraft::StorageIOError::read_logs(&e),
            })? {
                Some(bytes) => serde_json::from_slice::<LogId<NodeId>>(&bytes).ok(),
                None => None,
            }
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
        let meta_cf = self.meta_cf();
        let bytes = serde_json::to_vec(&committed).map_err(|e| StorageError::IO {
            source: openraft::StorageIOError::write_logs(&e),
        })?;
        self.db
            .put_cf(meta_cf, b"committed", bytes)
            .map_err(|e| StorageError::IO {
                source: openraft::StorageIOError::write_logs(&e),
            })
    }

    async fn read_committed(&mut self) -> Result<Option<LogId<NodeId>>, StorageError<NodeId>> {
        let meta_cf = self.meta_cf();
        match self.db.get_cf(meta_cf, b"committed").map_err(|e| StorageError::IO {
            source: openraft::StorageIOError::read_logs(&e),
        })? {
            Some(bytes) => {
                let v = serde_json::from_slice(&bytes).map_err(|e| StorageError::IO {
                    source: openraft::StorageIOError::read_logs(&e),
                })?;
                Ok(v)
            }
            None => Ok(None),
        }
    }

    async fn save_vote(&mut self, vote: &Vote<NodeId>) -> Result<(), StorageError<NodeId>> {
        let meta_cf = self.meta_cf();
        let bytes = serde_json::to_vec(vote).map_err(|e| StorageError::IO {
            source: openraft::StorageIOError::write_vote(&e),
        })?;
        self.db
            .put_cf(meta_cf, b"vote", bytes)
            .map_err(|e| StorageError::IO {
                source: openraft::StorageIOError::write_vote(&e),
            })
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<NodeId>>, StorageError<NodeId>> {
        let meta_cf = self.meta_cf();
        match self.db.get_cf(meta_cf, b"vote").map_err(|e| StorageError::IO {
            source: openraft::StorageIOError::read_vote(&e),
        })? {
            Some(bytes) => {
                let vote: Vote<NodeId> = serde_json::from_slice(&bytes).map_err(|e| {
                    StorageError::IO {
                        source: openraft::StorageIOError::read_vote(&e),
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
        let cf = self.logs_cf();

        for entry in entries {
            let lid = *entry.get_log_id();
            let bytes = serde_json::to_vec(&entry).map_err(|e| StorageError::IO {
                source: openraft::StorageIOError::write_logs(&e),
            })?;
            self.db
                .put_cf(cf, log_key(lid.index), bytes)
                .map_err(|e| StorageError::IO {
                    source: openraft::StorageIOError::write_logs(&e),
                })?;
        }

        callback.log_io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        let state = self.get_log_state().await?;
        let last_index = state.last_log_id.map(|l| l.index).unwrap_or(0);
        let cf = self.logs_cf();

        for idx in log_id.index..=last_index {
            self.db.delete_cf(cf, log_key(idx)).map_err(|e| StorageError::IO {
                source: openraft::StorageIOError::write_logs(&e),
            })?;
        }
        Ok(())
    }

    async fn purge(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        let cf = self.logs_cf();
        for idx in 0..=log_id.index {
            let _ = self.db.delete_cf(cf, log_key(idx));
        }

        let meta_cf = self.meta_cf();
        let bytes = serde_json::to_vec(&log_id).map_err(|e| StorageError::IO {
            source: openraft::StorageIOError::write_logs(&e),
        })?;
        self.db
            .put_cf(meta_cf, b"purged", bytes)
            .map_err(|e| StorageError::IO {
                source: openraft::StorageIOError::write_logs(&e),
            })?;
        Ok(())
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        LogStoreReader {
            db: self.db.clone(),
        }
    }
}
