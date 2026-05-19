//! Journal Implementations - Pluggable Event Persistence
//!
//! Like Akka Persistence, you can switch storage backends by configuration:
//! - InMemoryJournal: For development/testing
//! - CassandraJournal: For production (TODO)
//! - PostgreSQLJournal: Alternative production option (TODO)

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use rocksdb::DB;
use tokio::sync::RwLock;

use crate::{
    adapter::storage::EventStoreFactory,
    domain::{
        error::{StorageError, WorkflowError},
        event::{AggregateEvent, EventMetadata, WorkflowEvent},
        state::WorkflowState
    },
    port::{event::Event, journal::Journal},
    t, t_params
};

/// Width to which snapshot sequences are zero-padded in RocksDB keys.
/// u64::MAX is 20 decimal digits, so 20 is enough for any sequence and lets
/// `seek_for_prev` work correctly (lexicographic order = numeric order).
const SNAPSHOT_SEQ_WIDTH: usize = 20;

/// Build the key used to store a snapshot of `session_id` at the given sequence.
/// Format: `snapshot:{session_id}:{0-padded sequence}`.
fn snapshot_key(session_id: &str, sequence: u64) -> String {
    format!("snapshot:{}:{:0>width$}", session_id, sequence, width = SNAPSHOT_SEQ_WIDTH)
}

/// In-Memory Journal Implementation
///
/// Simple HashMap-based storage for development and testing.
/// Events are stored by session_id (persistence_id).
pub struct InMemoryJournal {
    /// Events stored by session_id -> Vec<AggregateEvent>
    events: Arc<RwLock<HashMap<String, Vec<AggregateEvent>>>>
}

impl InMemoryJournal {
    pub fn new() -> Self {
        Self { events: Arc::new(RwLock::new(HashMap::new())) }
    }
}

impl Default for InMemoryJournal {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Journal for InMemoryJournal {
    async fn persist_events(&self, session_id: &str, events: &[WorkflowEvent]) -> Result<(), WorkflowError> {
        if events.is_empty() {
            return Ok(());
        }

        let mut store = self.events.write().await;
        let session_events = store.entry(session_id.to_string()).or_insert_with(Vec::new);

        for event in events {
            let aggregate_event = AggregateEvent {
                aggregate_id: Some(session_id.to_string()),
                data:         event.clone(),
                metadata:     Some(EventMetadata::new(event.to_string()).with_aggregate_id(session_id))
            };
            session_events.push(aggregate_event);
        }

        Ok(())
    }

    async fn replay_events(&self, session_id: &str, from_sequence: u64) -> Result<Vec<WorkflowEvent>, WorkflowError> {
        let store = self.events.read().await;

        if let Some(session_events) = store.get(session_id) {
            let events = session_events.iter().skip(from_sequence as usize).map(|ae| ae.data.clone()).collect();
            Ok(events)
        } else {
            Ok(vec![])
        }
    }

    async fn highest_sequence_nr(&self, session_id: &str) -> Result<u64, WorkflowError> {
        let store = self.events.read().await;

        if let Some(session_events) = store.get(session_id) { Ok(session_events.len() as u64) } else { Ok(0) }
    }

    async fn delete_events(&self, session_id: &str, to_sequence: u64) -> Result<(), WorkflowError> {
        let mut store = self.events.write().await;

        if let Some(session_events) = store.get_mut(session_id) {
            session_events.drain(0..to_sequence.min(session_events.len() as u64) as usize);
        }

        Ok(())
    }

    async fn load_snapshot(&self, _session_id: &str) -> Result<Option<(u64, WorkflowState)>, WorkflowError> {
        Ok(None)
    }
}

pub struct RocksDbJournal {
    db:                 Arc<DB>,
    snapshot_threshold: u64
}

impl RocksDbJournal {
    /// Creates a new RocksDB journal from an existing DB instance
    ///
    /// Uses a shared RocksDB instance with the EventStore to avoid locking issues.
    pub fn new(db: Arc<DB>) -> Self {
        Self::with_snapshot_threshold(db, 100)
    }

    /// Creates a new RocksDB journal with custom snapshot threshold
    pub fn with_snapshot_threshold(db: Arc<DB>, snapshot_threshold: u64) -> Self {
        Self { db, snapshot_threshold }
    }

    /// Creates a snapshot of the current state of the journal
    async fn create_snapshot(
        &self,
        session_id: &str,
        sequence: u64,
        events: &[WorkflowEvent]
    ) -> Result<(), WorkflowError> {
        let session_id = session_id.to_string();
        let db = self.db.clone();
        let events = events.to_vec();

        tokio::task::spawn_blocking(move || -> Result<(), WorkflowError> {
            let mut state = WorkflowState::default();
            for event in events {
                if let Some(new_state) = event.apply(Some(&state)) {
                    state = new_state;
                }
            }

            let key = snapshot_key(&session_id, sequence);
            let serialized = serde_json::to_vec(&state)
                .map_err(|e| StorageError::Serialization(format!("Failed to serialize snapshot: {}", e)))?;

            db.put(key.as_bytes(), serialized)
                .map_err(|e| StorageError::Io(format!("Failed to write snapshot: {}", e)))?;

            Ok(())
        })
        .await
        .map_err(|e| WorkflowError::from(StorageError::Io(format!("Failed to create snapshot: {}", e))))??;

        Ok(())
    }
}

#[async_trait]
impl Journal for RocksDbJournal {
    async fn persist_events(&self, session_id: &str, events: &[WorkflowEvent]) -> Result<(), WorkflowError> {
        if events.is_empty() {
            return Ok(());
        }

        let session_id_owned = session_id.to_string();
        let db = self.db.clone();
        let events_to_store: Vec<AggregateEvent> = events
            .iter()
            .map(|event| AggregateEvent {
                aggregate_id: Some(session_id_owned.clone()),
                data:         event.clone(),
                metadata:     Some(EventMetadata::new(event.to_string()).with_aggregate_id(&session_id_owned))
            })
            .collect();

        tokio::task::spawn_blocking(move || -> Result<(), WorkflowError> {
            let key = format!("journal:{}", session_id_owned);

            let mut all_events: Vec<AggregateEvent> = match db.get(key.as_bytes()) {
                Ok(Some(data)) => serde_json::from_slice(&data)
                    .map_err(|e| StorageError::Serialization(format!("Failed to deserialize journal events: {}", e)))?,
                Ok(None) => vec![],
                Err(e) => return Err(StorageError::Io(format!("Failed to read journal: {}", e)).into())
            };

            all_events.extend(events_to_store);

            let serialized = serde_json::to_vec(&all_events)
                .map_err(|e| StorageError::Serialization(format!("Failed to serialize journal events: {}", e)))?;

            db.put(key.as_bytes(), serialized)
                .map_err(|e| StorageError::Io(format!("Failed to write journal: {}", e)))?;

            Ok(())
        })
        .await
        .map_err(|e| WorkflowError::from(StorageError::Io(format!("Failed to persist journal events: {}", e))))??;

        let sequence = self.highest_sequence_nr(session_id).await?;
        if sequence % self.snapshot_threshold == 0 {
            let all_events = tokio::task::spawn_blocking({
                let db = self.db.clone();
                let session_id = session_id.to_string();
                move || -> Result<Vec<WorkflowEvent>, WorkflowError> {
                    let key = format!("journal:{}", session_id);
                    match db.get(key.as_bytes()) {
                        Ok(Some(data)) => {
                            let aggregate_events: Vec<AggregateEvent> = serde_json::from_slice(&data)
                                .map_err(|e| StorageError::Serialization(format!("Failed to deserialize: {}", e)))?;
                            Ok(aggregate_events.into_iter().map(|ae| ae.data).collect())
                        }
                        Ok(None) => Ok(vec![]),
                        Err(e) => Err(StorageError::Io(format!("Failed to read: {}", e)).into())
                    }
                }
            })
            .await
            .map_err(|e| WorkflowError::from(StorageError::Io(format!("Failed to read events: {}", e))))??;

            self.create_snapshot(session_id, sequence, &all_events).await?;
        }

        Ok(())
    }

    async fn replay_events(&self, session_id: &str, from_sequence: u64) -> Result<Vec<WorkflowEvent>, WorkflowError> {
        let session_id = session_id.to_string();
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<WorkflowEvent>, WorkflowError> {
            let key = format!("journal:{}", session_id);

            match db.get(key.as_bytes()) {
                Ok(Some(data)) => {
                    let aggregate_events: Vec<AggregateEvent> = serde_json::from_slice(&data).map_err(|e| {
                        StorageError::Serialization(format!("Failed to deserialize journal events: {}", e))
                    })?;
                    Ok(aggregate_events.into_iter().skip(from_sequence as usize).map(|ae| ae.data).collect())
                }
                Ok(None) => Ok(vec![]),
                Err(e) => Err(StorageError::Io(format!("Failed to read journal: {}", e)).into())
            }
        })
        .await
        .map_err(|e| WorkflowError::from(StorageError::Io(format!("Failed to replay journal events: {}", e))))?
    }

    async fn highest_sequence_nr(&self, session_id: &str) -> Result<u64, WorkflowError> {
        let session_id = session_id.to_string();
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || -> Result<u64, WorkflowError> {
            let key = format!("journal:{}", session_id);

            match db.get(key.as_bytes()) {
                Ok(Some(data)) => {
                    let aggregate_events: Vec<AggregateEvent> = serde_json::from_slice(&data).map_err(|e| {
                        StorageError::Serialization(format!("Failed to deserialize journal events: {}", e))
                    })?;
                    Ok(aggregate_events.len() as u64)
                }
                Ok(None) => Ok(0),
                Err(e) => Err(StorageError::Io(format!("Failed to read journal: {}", e)).into())
            }
        })
        .await
        .map_err(|e| WorkflowError::Storage(StorageError::Io(format!("Failed to get sequence number: {}", e))))?
    }

    async fn delete_events(&self, session_id: &str, to_sequence: u64) -> Result<(), WorkflowError> {
        let session_id = session_id.to_string();
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || -> Result<(), WorkflowError> {
            let key = format!("journal:{}", session_id);

            // `persist_events` writes `Vec<AggregateEvent>`; round-trip the same type so the
            // wrapper (and its metadata) isn't silently dropped on rewrite.
            match db.get(key.as_bytes()) {
                Ok(Some(data)) => {
                    let mut events: Vec<AggregateEvent> = serde_json::from_slice(&data).map_err(|e| {
                        StorageError::Serialization(format!("Failed to deserialize journal events: {}", e))
                    })?;

                    events.drain(0..(to_sequence as usize).min(events.len()));

                    if events.is_empty() {
                        db.delete(key.as_bytes())
                            .map_err(|e| StorageError::Io(format!("Failed to delete journal: {}", e)))?;
                    } else {
                        let serialized = serde_json::to_vec(&events).map_err(|e| {
                            StorageError::Serialization(format!("Failed to serialize journal events: {}", e))
                        })?;
                        db.put(key.as_bytes(), serialized)
                            .map_err(|e| StorageError::Io(format!("Failed to write journal: {}", e)))?;
                    }
                }
                Ok(None) => {}
                Err(e) => return Err(StorageError::Io(format!("Failed to read journal: {}", e)).into())
            }

            Ok(())
        })
        .await
        .map_err(|e| {
            WorkflowError::Storage(StorageError::Io(t_params!(
                "error_failed_to_delete_journal_events",
                &[&e.to_string()]
            )))
        })?
    }

    async fn load_snapshot(&self, session_id: &str) -> Result<Option<(u64, WorkflowState)>, WorkflowError> {
        let session_id = session_id.to_string();
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || -> Result<Option<(u64, WorkflowState)>, WorkflowError> {
            // Snapshot keys are zero-padded so lexicographic order matches numeric order:
            // seek_for_prev with the MAX-sequence key returns the highest snapshot for the
            // session in O(log n).
            let prefix = format!("snapshot:{}:", session_id);
            let seek_key = snapshot_key(&session_id, u64::MAX);

            let mut iter = db.raw_iterator();
            iter.seek_for_prev(seek_key.as_bytes());

            if !iter.valid() {
                return Ok(None);
            }

            let Some(key_bytes) = iter.key() else { return Ok(None) };
            if !key_bytes.starts_with(prefix.as_bytes()) {
                return Ok(None);
            }

            let key_str = std::str::from_utf8(key_bytes)
                .map_err(|e| StorageError::Serialization(format!("Snapshot key is not valid UTF-8: {}", e)))?;
            let seq_str = &key_str[prefix.len()..];
            let sequence = seq_str
                .parse::<u64>()
                .map_err(|e| StorageError::Serialization(format!("Invalid snapshot sequence: {}", e)))?;

            let value_bytes =
                iter.value().ok_or_else(|| StorageError::Io(format!("Snapshot {} has no value", key_str)))?;
            let state: WorkflowState = serde_json::from_slice(value_bytes)
                .map_err(|e| StorageError::Serialization(format!("Failed to deserialize snapshot state: {}", e)))?;

            Ok(Some((sequence, state)))
        })
        .await
        .map_err(|e| WorkflowError::from(StorageError::Io(format!("Failed to load snapshot: {}", e))))?
    }
}

/// Journal Factory - Configuration-driven journal creation
pub enum JournalType {
    InMemory,
    RocksDb
}

pub struct JournalFactory;

impl JournalFactory {
    pub fn create(journal_type: JournalType) -> Result<Arc<dyn Journal>, WorkflowError> {
        match journal_type {
            JournalType::InMemory => Ok(Arc::new(InMemoryJournal::new())),
            JournalType::RocksDb => {
                let db =
                    EventStoreFactory::get_db().ok_or(WorkflowError::Config(t!("error_rocksdb_not_initialized")))?;
                Ok(Arc::new(RocksDbJournal::new(db)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use rocksdb::{DB, Options};
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;
    use crate::domain::{event::*, workflow::Workflow};

    #[tokio::test]
    async fn test_inmemory_journal() {
        let journal = InMemoryJournal::new();
        let session_id = "test-session";

        // Initially empty
        let events = journal.replay_events(session_id, 0).await.unwrap();
        assert!(events.is_empty());
        assert_eq!(journal.highest_sequence_nr(session_id).await.unwrap(), 0);

        // Persist some events
        let test_events = vec![WorkflowEvent::SyncRequested(SyncRequestedEvent {
            event_id:   Uuid::new_v4().to_string(),
            timestamp:  Utc::now(),
            remote_url: "test-url".to_string(),
            branch:     "main".to_string(),
            ssh_key:    None
        })];

        journal.persist_events(session_id, &test_events).await.unwrap();

        // Verify persistence
        let replayed = journal.replay_events(session_id, 0).await.unwrap();
        assert_eq!(replayed.len(), 1);
        assert_eq!(journal.highest_sequence_nr(session_id).await.unwrap(), 1);

        // Test deletion
        journal.delete_events(session_id, 1).await.unwrap();
        let after_delete = journal.replay_events(session_id, 0).await.unwrap();
        assert!(after_delete.is_empty());
    }

    #[tokio::test]
    async fn inmemory_journal_load_snapshot_returns_none() {
        let journal = InMemoryJournal::new();
        assert!(journal.load_snapshot("any-session").await.unwrap().is_none());
    }

    /// Build a temp RocksDB and RocksDbJournal with a low snapshot threshold for testing.
    fn temp_rocksdb_journal(snapshot_threshold: u64) -> (TempDir, RocksDbJournal) {
        let temp = tempfile::tempdir().expect("create tempdir");
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, temp.path()).expect("open rocksdb");
        let journal = RocksDbJournal::with_snapshot_threshold(Arc::new(db), snapshot_threshold);
        (temp, journal)
    }

    fn discovered_event(name: &str) -> WorkflowEvent {
        WorkflowEvent::WorkflowDiscovered(WorkflowDiscoveredEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow:  Workflow {
                name:        name.to_string(),
                command:     "echo".to_string(),
                description: "test".to_string(),
                arguments:   vec![],
                tags:        vec![],
                source_url:  None,
                author:      None,
                author_url:  None,
                shells:      vec![]
            },
            file_path: format!("{}.yaml", name)
        })
    }

    #[tokio::test]
    async fn rocksdb_journal_load_snapshot_none_before_threshold() {
        let (_tmp, journal) = temp_rocksdb_journal(10);
        let session_id = "session-A";

        // Persist a single event — under the threshold of 10.
        journal.persist_events(session_id, &[discovered_event("wf-1")]).await.unwrap();

        assert!(
            journal.load_snapshot(session_id).await.unwrap().is_none(),
            "snapshot should not exist before threshold is reached"
        );
    }

    #[tokio::test]
    async fn rocksdb_journal_creates_snapshot_at_threshold() {
        // Threshold = 2 means snapshot is taken every 2 persisted events.
        let (_tmp, journal) = temp_rocksdb_journal(2);
        let session_id = "session-B";

        journal.persist_events(session_id, &[discovered_event("wf-1")]).await.unwrap();
        journal.persist_events(session_id, &[discovered_event("wf-2")]).await.unwrap();

        let snapshot = journal.load_snapshot(session_id).await.unwrap();
        assert!(snapshot.is_some(), "expected a snapshot after reaching threshold");
        let (sequence, _state) = snapshot.unwrap();
        assert_eq!(sequence, 2, "snapshot should be at sequence 2 after 2 events");
    }

    #[tokio::test]
    async fn rocksdb_journal_replay_from_snapshot_returns_only_subsequent_events() {
        // Regression for the original bug: replay_events used to implicitly skip past
        // the snapshot sequence even when callers didn't load it. Now callers explicitly
        // pass from_sequence — we verify it slices correctly and the new contract holds:
        // replay_events(0) returns ALL events (no implicit skip).
        let (_tmp, journal) = temp_rocksdb_journal(2);
        let session_id = "session-C";

        // Snapshot is taken when highest_sequence_nr % threshold == 0.
        // Persist 3 events: snapshot triggers at sequence 2.
        journal.persist_events(session_id, &[discovered_event("wf-1")]).await.unwrap();
        journal.persist_events(session_id, &[discovered_event("wf-2")]).await.unwrap();
        journal.persist_events(session_id, &[discovered_event("wf-3")]).await.unwrap();

        // replay_events(0) should return all 3 events (no implicit skip).
        let all = journal.replay_events(session_id, 0).await.unwrap();
        assert_eq!(all.len(), 3, "replay_events(0) must return every persisted event");

        // load_snapshot returns the latest snapshot (at seq 2 since threshold is 2 and
        // we only crossed a multiple of 2 once — at seq=2).
        let (snapshot_seq, _) = journal.load_snapshot(session_id).await.unwrap().unwrap();
        assert_eq!(snapshot_seq, 2, "snapshot should be at the most recent multiple of threshold");

        // replay_events from the snapshot sequence returns only the events after it.
        let after_snapshot = journal.replay_events(session_id, snapshot_seq).await.unwrap();
        assert_eq!(after_snapshot.len(), 1, "expected 1 event after snapshot at seq {}", snapshot_seq);
    }

    #[tokio::test]
    async fn rocksdb_journal_load_snapshot_returns_latest_when_multiple_exist() {
        // With threshold=2 and 4 events, snapshots are taken at seq=2 and seq=4.
        // load_snapshot must return the latest (seq=4), not the first.
        let (_tmp, journal) = temp_rocksdb_journal(2);
        let session_id = "session-E";

        for i in 1..=4 {
            journal.persist_events(session_id, &[discovered_event(&format!("wf-{}", i))]).await.unwrap();
        }

        let (sequence, _) = journal.load_snapshot(session_id).await.unwrap().unwrap();
        assert_eq!(sequence, 4, "should return the highest snapshot sequence, not an older one");
    }

    #[test]
    fn snapshot_key_sorts_lexicographically_in_numeric_order() {
        // The whole point of zero-padding is that seek_for_prev works correctly.
        // If a non-padded "2" key existed it would sort after "10" lexicographically.
        let k2 = snapshot_key("S", 2);
        let k10 = snapshot_key("S", 10);
        let k100 = snapshot_key("S", 100);
        let k_max = snapshot_key("S", u64::MAX);

        assert!(k2 < k10, "{} should sort before {}", k2, k10);
        assert!(k10 < k100, "{} should sort before {}", k10, k100);
        assert!(k100 < k_max, "{} should sort before {}", k100, k_max);
    }

    #[tokio::test]
    async fn rocksdb_journal_snapshot_state_reflects_applied_events() {
        // After a snapshot is taken, loading it should yield a non-default WorkflowState
        // because at least one event has been applied. (The previous bug was that the
        // recovery path never loaded the snapshot at all, so this guards against
        // regressing back to default state on recovery.)
        use crate::domain::state::WorkflowState;

        let (_tmp, journal) = temp_rocksdb_journal(2);
        let session_id = "session-D";

        journal.persist_events(session_id, &[discovered_event("wf-1")]).await.unwrap();
        journal.persist_events(session_id, &[discovered_event("wf-2")]).await.unwrap();

        let (_, state) = journal.load_snapshot(session_id).await.unwrap().expect("snapshot present");
        assert!(
            !matches!(state, WorkflowState::Initial(_)),
            "snapshot state should reflect applied WorkflowDiscovered events, not be Initial"
        );
    }
}
