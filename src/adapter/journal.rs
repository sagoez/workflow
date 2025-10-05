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
        error::WorkflowError,
        event::{AggregateEvent, EventMetadata, WorkflowEvent}
    },
    port::journal::Journal,
    t
};

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
        use crate::{domain::state::WorkflowState, port::event::Event};

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

            let key = format!("snapshot:{}:{}", session_id, sequence);
            let serialized = serde_json::to_vec(&state)
                .map_err(|e| WorkflowError::Serialization(format!("Failed to serialize snapshot: {}", e)))?;

            db.put(key.as_bytes(), serialized)
                .map_err(|e| WorkflowError::FileSystem(format!("Failed to write snapshot: {}", e)))?;

            Ok(())
        })
        .await
        .map_err(|e| WorkflowError::Generic(format!("Failed to create snapshot: {}", e)))??;

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
                Ok(Some(data)) => serde_json::from_slice(&data).map_err(|e| {
                    WorkflowError::Serialization(format!("Failed to deserialize journal events: {}", e))
                })?,
                Ok(None) => vec![],
                Err(e) => return Err(WorkflowError::FileSystem(format!("Failed to read journal: {}", e)))
            };

            all_events.extend(events_to_store);

            let serialized = serde_json::to_vec(&all_events)
                .map_err(|e| WorkflowError::Serialization(format!("Failed to serialize journal events: {}", e)))?;

            db.put(key.as_bytes(), serialized)
                .map_err(|e| WorkflowError::FileSystem(format!("Failed to write journal: {}", e)))?;

            Ok(())
        })
        .await
        .map_err(|e| WorkflowError::Generic(format!("Failed to persist journal events: {}", e)))??;

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
                                .map_err(|e| WorkflowError::Serialization(format!("Failed to deserialize: {}", e)))?;
                            Ok(aggregate_events.into_iter().map(|ae| ae.data).collect())
                        }
                        Ok(None) => Ok(vec![]),
                        Err(e) => Err(WorkflowError::FileSystem(format!("Failed to read: {}", e)))
                    }
                }
            })
            .await
            .map_err(|e| WorkflowError::Generic(format!("Failed to read events: {}", e)))??;

            self.create_snapshot(session_id, sequence, &all_events).await?;
        }

        Ok(())
    }

    async fn replay_events(&self, session_id: &str, from_sequence: u64) -> Result<Vec<WorkflowEvent>, WorkflowError> {
        let session_id_owned = session_id.to_string();
        let db = self.db.clone();

        let snapshot_sequence = tokio::task::spawn_blocking(move || -> Result<Option<u64>, WorkflowError> {
            let snapshot_prefix = format!("snapshot:{}:", session_id_owned);
            let seek_key = format!("snapshot:{}:{}", session_id_owned, u64::MAX);

            let mut iter = db.raw_iterator();
            iter.seek_for_prev(seek_key.as_bytes());

            if iter.valid() {
                if let Some(key_bytes) = iter.key() {
                    let key_str = String::from_utf8_lossy(key_bytes);

                    if key_str.starts_with(&snapshot_prefix) {
                        let parts: Vec<&str> = key_str.split(':').collect();
                        if parts.len() == 3 {
                            let sequence = parts[2]
                                .parse::<u64>()
                                .map_err(|e| WorkflowError::Serialization(format!("Invalid sequence: {}", e)))?;
                            return Ok(Some(sequence));
                        }
                    }
                }
            }

            Ok(None)
        })
        .await
        .map_err(|e| WorkflowError::Generic(format!("Failed to find snapshot: {}", e)))??;

        let skip_until = snapshot_sequence.unwrap_or(0).max(from_sequence);

        let session_id = session_id.to_string();
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<WorkflowEvent>, WorkflowError> {
            let key = format!("journal:{}", session_id);

            match db.get(key.as_bytes()) {
                Ok(Some(data)) => {
                    let aggregate_events: Vec<AggregateEvent> = serde_json::from_slice(&data).map_err(|e| {
                        WorkflowError::Serialization(format!("Failed to deserialize journal events: {}", e))
                    })?;
                    Ok(aggregate_events.into_iter().skip(skip_until as usize).map(|ae| ae.data).collect())
                }
                Ok(None) => Ok(vec![]),
                Err(e) => Err(WorkflowError::FileSystem(format!("Failed to read journal: {}", e)))
            }
        })
        .await
        .map_err(|e| WorkflowError::Generic(format!("Failed to replay journal events: {}", e)))?
    }

    async fn highest_sequence_nr(&self, session_id: &str) -> Result<u64, WorkflowError> {
        let session_id = session_id.to_string();
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || -> Result<u64, WorkflowError> {
            let key = format!("journal:{}", session_id);

            match db.get(key.as_bytes()) {
                Ok(Some(data)) => {
                    let aggregate_events: Vec<AggregateEvent> = serde_json::from_slice(&data).map_err(|e| {
                        WorkflowError::Serialization(format!("Failed to deserialize journal events: {}", e))
                    })?;
                    Ok(aggregate_events.len() as u64)
                }
                Ok(None) => Ok(0),
                Err(e) => Err(WorkflowError::FileSystem(format!("Failed to read journal: {}", e)))
            }
        })
        .await
        .map_err(|e| WorkflowError::Generic(format!("Failed to get sequence number: {}", e)))?
    }

    async fn delete_events(&self, session_id: &str, to_sequence: u64) -> Result<(), WorkflowError> {
        let session_id = session_id.to_string();
        let db = self.db.clone();

        tokio::task::spawn_blocking(move || -> Result<(), WorkflowError> {
            let key = format!("journal:{}", session_id);

            match db.get(key.as_bytes()) {
                Ok(Some(data)) => {
                    let mut events: Vec<WorkflowEvent> = serde_json::from_slice(&data).map_err(|e| {
                        WorkflowError::Serialization(format!("Failed to deserialize journal events: {}", e))
                    })?;

                    events.drain(0..(to_sequence as usize).min(events.len()));

                    if events.is_empty() {
                        db.delete(key.as_bytes())
                            .map_err(|e| WorkflowError::FileSystem(format!("Failed to delete journal: {}", e)))?;
                    } else {
                        let serialized = serde_json::to_vec(&events).map_err(|e| {
                            WorkflowError::Serialization(format!("Failed to serialize journal events: {}", e))
                        })?;
                        db.put(key.as_bytes(), serialized)
                            .map_err(|e| WorkflowError::FileSystem(format!("Failed to write journal: {}", e)))?;
                    }
                }
                Ok(None) => {}
                Err(e) => return Err(WorkflowError::FileSystem(format!("Failed to read journal: {}", e)))
            }

            Ok(())
        })
        .await
        .map_err(|e| WorkflowError::Generic(format!("Failed to delete journal events: {}", e)))?
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
                    EventStoreFactory::get_db().ok_or(WorkflowError::Generic(t!("error_rocksdb_not_initialized")))?;
                Ok(Arc::new(RocksDbJournal::new(db)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;
    use crate::domain::event::*;

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
}
