//! Storage implementations for events and state
//!
//! This module provides both in-memory and persistent (RocksDB) implementations
//! of the EventStore trait.

use std::{collections::HashMap, path::Path, sync::Arc};

use async_trait::async_trait;
use rocksdb::{DB, Options};
use tokio::sync::RwLock;

use crate::{
    domain::{
        error::WorkflowError,
        event::{AggregateEvent, EventMetadata, WorkflowEvent},
        state::WorkflowState
    },
    port::{event::Event, storage::EventStore},
    t
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, clap::ValueEnum)]
pub enum EventStoreType {
    #[serde(rename = "inmemory")]
    #[value(name = "inmemory")]
    InMemory,
    #[serde(rename = "rocksdb")]
    #[value(name = "rocksdb")]
    RocksDb
}

impl EventStoreType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStoreType::InMemory => "inmemory",
            EventStoreType::RocksDb => "rocksdb"
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "inmemory" => Ok(EventStoreType::InMemory),
            "rocksdb" => Ok(EventStoreType::RocksDb),
            other => Err(format!("Unknown storage backend: {}", other))
        }
    }
}

/// In-memory event store implementation
/// Stores events and maintains state per aggregate (session_id).
/// This is suitable for development and testing, but not for production
/// as data is lost when the application restarts.
#[derive(Debug, Default)]
pub struct InMemoryEventStore {
    /// Aggregate events stored per aggregate_id (session_id)
    events: Arc<RwLock<HashMap<String, Vec<AggregateEvent>>>>,
    /// Cached state per aggregate_id for performance
    cache:  Arc<RwLock<HashMap<String, WorkflowState>>>
}

impl InMemoryEventStore {
    /// Create a new in-memory event store
    pub fn new() -> Self {
        Self { events: Arc::new(RwLock::new(HashMap::new())), cache: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Apply events to state to rebuild current state
    async fn rebuild_state(&self, aggregate_id: &str) -> Result<WorkflowState, WorkflowError> {
        let events = self.events.read().await;
        let aggregate_events = events.get(aggregate_id).cloned().unwrap_or_default();
        drop(events);

        let mut current_state = WorkflowState::default();

        for aggregate_event in aggregate_events {
            if let Some(new_state) = aggregate_event.data.apply(Some(&current_state)) {
                current_state = new_state;
            } else {
                return Err(WorkflowError::Validation(t!("error_failed_to_apply_event")));
            }
        }

        let mut cache = self.cache.write().await;
        cache.insert(aggregate_id.to_string(), current_state.clone());

        Ok(current_state)
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    /// Store multiple workflow events for a specific session/aggregate
    async fn store_events(&self, session_id: &str, events: &[WorkflowEvent]) -> Result<(), WorkflowError> {
        if events.is_empty() {
            return Ok(());
        }

        let aggregate_id = session_id.to_string();

        let mut event_store = self.events.write().await;
        let aggregate_events = event_store.entry(aggregate_id.clone()).or_insert_with(Vec::new);

        for event in events {
            let metadata = EventMetadata::new(event.to_string()).with_aggregate_id(&aggregate_id);

            let aggregate_event = AggregateEvent {
                aggregate_id: Some(aggregate_id.clone()),
                data:         event.clone(),
                metadata:     Some(metadata)
            };

            aggregate_events.push(aggregate_event);
        }

        drop(event_store);

        // Invalidate cached state so it gets rebuilt on next access
        let mut cache = self.cache.write().await;
        cache.remove(&aggregate_id);

        Ok(())
    }

    /// Get current state for a specific aggregate (session_id)
    async fn get_current_state(&self, aggregate_id: &str) -> Result<WorkflowState, WorkflowError> {
        {
            let cache = self.cache.read().await;
            if let Some(cached_state) = cache.get(aggregate_id) {
                return Ok(cached_state.clone());
            }
        }

        self.rebuild_state(aggregate_id).await
    }

    async fn get_events(&self, session_id: &str) -> Result<Vec<WorkflowEvent>, WorkflowError> {
        let event_store = self.events.read().await;

        if let Some(aggregate_events) = event_store.get(session_id) {
            let workflow_events: Vec<WorkflowEvent> = aggregate_events.iter().map(|ae| ae.data.clone()).collect();
            Ok(workflow_events)
        } else {
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;
    use crate::domain::{event::*, workflow::Workflow};

    fn create_test_workflow() -> Workflow {
        Workflow {
            name:        "test-workflow".to_string(),
            description: "Test workflow".to_string(),
            command:     "echo 'test'".to_string(),
            arguments:   vec![],
            source_url:  None,
            author:      None,
            author_url:  None,
            shells:      vec![],
            tags:        vec![]
        }
    }

    #[tokio::test]
    async fn test_store_and_retrieve_events() {
        let store = InMemoryEventStore::new();
        let workflow = create_test_workflow();

        let events = vec![
            WorkflowEvent::WorkflowDiscovered(WorkflowDiscoveredEvent {
                event_id:  Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                workflow:  workflow.clone(),
                file_path: "test.yaml".to_string()
            }),
            WorkflowEvent::AvailableWorkflowsListed(AvailableWorkflowsListedEvent {
                event_id:  Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                workflows: vec!["test-workflow".to_string()]
            }),
            WorkflowEvent::WorkflowSelected(WorkflowSelectedEvent {
                event_id:  Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                workflow:  workflow.clone(),
                user:      "test_user".to_string()
            }),
        ];

        // Store events
        store.store_events("test_session", &events).await.unwrap();

        // Retrieve state
        let state = store.get_current_state("test_session").await.unwrap();

        // Should be in WorkflowSelected state after both events
        match state {
            WorkflowState::WorkflowSelected(selected_state) => {
                assert_eq!(selected_state.discovered_workflows.len(), 1);
                assert_eq!(selected_state.discovered_workflows[0].name, "test-workflow");
                assert_eq!(selected_state.selected_workflow.name, "test-workflow");
            }
            _ => panic!("Expected WorkflowSelected state")
        }
    }

    #[tokio::test]
    async fn test_workflow_lifecycle() {
        let store = InMemoryEventStore::new();
        let workflow = create_test_workflow();
        let session_id = "test_session";

        // 1. Discover workflow
        let discover_event = WorkflowEvent::WorkflowDiscovered(WorkflowDiscoveredEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow:  workflow.clone(),
            file_path: "test.yaml".to_string()
        });
        store.store_events(session_id, &[discover_event]).await.unwrap();

        let state = store.get_current_state(session_id).await.unwrap();
        match state {
            WorkflowState::WorkflowsDiscovered(discovered_state) => {
                assert_eq!(discovered_state.discovered_workflows.len(), 1);
            }
            _ => panic!("Expected WorkflowsDiscovered state")
        }

        // 2. List workflows
        let list_event = WorkflowEvent::AvailableWorkflowsListed(AvailableWorkflowsListedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflows: vec!["test-workflow".to_string()]
        });
        store.store_events(session_id, &[list_event]).await.unwrap();

        // 3. Select workflow
        let select_event = WorkflowEvent::WorkflowSelected(WorkflowSelectedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow:  workflow.clone(),
            user:      "test_user".to_string()
        });
        store.store_events(session_id, &[select_event]).await.unwrap();

        let state = store.get_current_state(session_id).await.unwrap();
        match state {
            WorkflowState::WorkflowSelected(selected_state) => {
                assert_eq!(selected_state.selected_workflow.name, "test-workflow");
            }
            _ => panic!("Expected WorkflowSelected state")
        }

        // 4. Start workflow
        let start_event = WorkflowEvent::WorkflowStarted(WorkflowStartedEvent {
            event_id:     Uuid::new_v4().to_string(),
            timestamp:    Utc::now(),
            user:         "test_user".to_string(),
            hostname:     "test_host".to_string(),
            execution_id: "exec_123".to_string()
        });
        store.store_events(session_id, &[start_event]).await.unwrap();

        let state = store.get_current_state(session_id).await.unwrap();
        match state {
            WorkflowState::WorkflowStarted(started_state) => {
                assert_eq!(started_state.execution_id, "exec_123");
            }
            _ => panic!("Expected WorkflowStarted state")
        }

        // 5. Resolve arguments
        let resolve_event = WorkflowEvent::WorkflowArgumentsResolved(WorkflowArgumentsResolvedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            arguments: std::collections::HashMap::new()
        });
        store.store_events(session_id, &[resolve_event]).await.unwrap();

        // 6. Complete workflow
        let complete_event = WorkflowEvent::WorkflowCompleted(WorkflowCompletedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now()
        });
        store.store_events(session_id, &[complete_event]).await.unwrap();

        let state = store.get_current_state(session_id).await.unwrap();
        match state {
            WorkflowState::WorkflowCompleted(completed_state) => {
                assert_eq!(completed_state.execution_id, "exec_123");
                assert_eq!(completed_state.completed_workflow.name, "test-workflow");
            }
            _ => panic!("Expected WorkflowCompleted state")
        }
    }
}

/// RocksDB-based event store implementation
/// Provides persistent storage for events and state across application restarts.
/// Uses RocksDB for high-performance key-value storage with efficient snapshots.
///
/// Storage layout:
/// - `events:{aggregate_id}` -> Vec<AggregateEvent> (all events)
/// - `snapshot:{aggregate_id}:{sequence}` -> WorkflowState (state snapshot at sequence)
/// - `seq:{aggregate_id}` -> u64 (current sequence number)
pub struct RocksDbEventStore {
    db:                 Arc<DB>,
    cache:              Arc<RwLock<HashMap<String, WorkflowState>>>,
    /// Number of events between snapshots (default: 10)
    snapshot_threshold: u64
}

// Note: RocksDbEventStore doesn't implement Default because it requires a valid file path
// Use RocksDbEventStore::new(path) instead

impl RocksDbEventStore {
    /// Create a new RocksDB event store at the specified path
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, WorkflowError> {
        Self::with_snapshot_threshold(path, 10)
    }

    /// Create a new RocksDB event store with custom snapshot threshold
    pub fn with_snapshot_threshold<P: AsRef<Path>>(path: P, snapshot_threshold: u64) -> Result<Self, WorkflowError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Snappy);

        let db =
            DB::open(&opts, path).map_err(|e| WorkflowError::FileSystem(format!("Failed to open RocksDB: {}", e)))?;

        Ok(Self { db: Arc::new(db), cache: Arc::new(RwLock::new(HashMap::new())), snapshot_threshold })
    }

    /// Apply events to state to rebuild current state using snapshots
    ///
    /// Algorithm:
    /// 1. Find latest snapshot (if any)
    /// 2. Load snapshot as starting state, or use default
    /// 3. Get events after snapshot sequence
    /// 4. Replay only those events
    /// 5. Cache the rebuilt state
    async fn rebuild_state(&self, aggregate_id: &str) -> Result<WorkflowState, WorkflowError> {
        let db = self.db.clone();
        let aggregate_id_owned = aggregate_id.to_string();

        let (starting_state, from_sequence) =
            tokio::task::spawn_blocking(move || -> Result<(WorkflowState, u64), WorkflowError> {
                let snapshot_prefix = format!("snapshot:{}:", aggregate_id_owned);
                let seek_key = format!("snapshot:{}:{}", aggregate_id_owned, u64::MAX);

                let mut iter = db.raw_iterator();
                iter.seek_for_prev(seek_key.as_bytes());

                if iter.valid() {
                    if let Some(key_bytes) = iter.key() {
                        let key_str = String::from_utf8_lossy(key_bytes);

                        if key_str.starts_with(&snapshot_prefix) {
                            let parts: Vec<&str> = key_str.split(':').collect();
                            if parts.len() == 3 {
                                let sequence = parts[2].parse::<u64>().map_err(|e| {
                                    WorkflowError::Serialization(format!("Invalid sequence in snapshot key: {}", e))
                                })?;

                                if let Some(value_bytes) = iter.value() {
                                    let state: WorkflowState = serde_json::from_slice(value_bytes).map_err(|e| {
                                        WorkflowError::Serialization(format!("Failed to deserialize snapshot: {}", e))
                                    })?;

                                    return Ok((state, sequence + 1));
                                }
                            }
                        }
                    }
                }

                Ok((WorkflowState::default(), 0))
            })
            .await
            .map_err(|e| WorkflowError::Generic(format!("Failed to load snapshot: {}", e)))??;

        let events = self.get_events_internal(aggregate_id).await?;
        let mut current_state = starting_state;

        for (idx, aggregate_event) in events.into_iter().enumerate() {
            if (idx as u64) >= from_sequence {
                if let Some(new_state) = aggregate_event.data.apply(Some(&current_state)) {
                    current_state = new_state;
                } else {
                    return Err(WorkflowError::Validation(t!("error_failed_to_apply_event")));
                }
            }
        }

        let mut cache = self.cache.write().await;
        cache.insert(aggregate_id.to_string(), current_state.clone());

        Ok(current_state)
    }

    /// Create a snapshot of the current state
    async fn create_snapshot(
        &self,
        aggregate_id: &str,
        sequence: u64,
        state: &WorkflowState
    ) -> Result<(), WorkflowError> {
        let db = self.db.clone();
        let aggregate_id = aggregate_id.to_string();
        let state = state.clone();

        tokio::task::spawn_blocking(move || -> Result<(), WorkflowError> {
            let key = format!("snapshot:{}:{}", aggregate_id, sequence);
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

    /// Get current sequence number for an aggregate
    async fn get_sequence(&self, aggregate_id: &str) -> Result<u64, WorkflowError> {
        let db = self.db.clone();
        let aggregate_id = aggregate_id.to_string();

        tokio::task::spawn_blocking(move || -> Result<u64, WorkflowError> {
            let key = format!("seq:{}", aggregate_id);
            match db.get(key.as_bytes()) {
                Ok(Some(data)) => {
                    let seq_bytes: [u8; 8] = data
                        .as_slice()
                        .try_into()
                        .map_err(|_| WorkflowError::Serialization("Invalid sequence number".to_string()))?;
                    Ok(u64::from_le_bytes(seq_bytes))
                }
                Ok(None) => Ok(0),
                Err(e) => Err(WorkflowError::FileSystem(format!("Failed to read sequence: {}", e)))
            }
        })
        .await
        .map_err(|e| WorkflowError::Generic(format!("Failed to get sequence: {}", e)))?
    }

    /// Update sequence number for an aggregate
    async fn set_sequence(&self, aggregate_id: &str, sequence: u64) -> Result<(), WorkflowError> {
        let db = self.db.clone();
        let aggregate_id = aggregate_id.to_string();

        tokio::task::spawn_blocking(move || -> Result<(), WorkflowError> {
            let key = format!("seq:{}", aggregate_id);
            db.put(key.as_bytes(), &sequence.to_le_bytes())
                .map_err(|e| WorkflowError::FileSystem(format!("Failed to write sequence: {}", e)))?;
            Ok(())
        })
        .await
        .map_err(|e| WorkflowError::Generic(format!("Failed to set sequence: {}", e)))?
    }

    /// Internal method to get events without using the trait
    async fn get_events_internal(&self, session_id: &str) -> Result<Vec<AggregateEvent>, WorkflowError> {
        let db = self.db.clone();
        let session_id = session_id.to_string();

        tokio::task::spawn_blocking(move || -> Result<Vec<AggregateEvent>, WorkflowError> {
            let key = format!("events:{}", session_id);

            match db.get(key.as_bytes()) {
                Ok(Some(data)) => {
                    let events: Vec<AggregateEvent> = serde_json::from_slice(&data)
                        .map_err(|e| WorkflowError::Serialization(format!("Failed to deserialize events: {}", e)))?;
                    Ok(events)
                }
                Ok(None) => Ok(vec![]),
                Err(e) => Err(WorkflowError::FileSystem(format!("Failed to read from RocksDB: {}", e)))
            }
        })
        .await
        .map_err(|e| WorkflowError::Generic(format!("Failed to read events: {}", e)))?
    }
}

#[async_trait]
impl EventStore for RocksDbEventStore {
    /// Store multiple workflow events for a specific session/aggregate
    async fn store_events(&self, session_id: &str, events: &[WorkflowEvent]) -> Result<(), WorkflowError> {
        if events.is_empty() {
            return Ok(());
        }

        let aggregate_id = session_id.to_string();
        let aggregate_id_for_blocking = aggregate_id.clone();
        let db = self.db.clone();
        let events_to_store = events.to_vec();

        tokio::task::spawn_blocking(move || -> Result<(), WorkflowError> {
            let key = format!("events:{}", aggregate_id_for_blocking);

            let mut all_events: Vec<AggregateEvent> = match db.get(key.as_bytes()) {
                Ok(Some(data)) => serde_json::from_slice(&data).map_err(|e| {
                    WorkflowError::Serialization(format!("Failed to deserialize existing events: {}", e))
                })?,
                Ok(None) => vec![],
                Err(e) => return Err(WorkflowError::FileSystem(format!("Failed to read from RocksDB: {}", e)))
            };

            for event in events_to_store {
                let metadata = EventMetadata::new(event.to_string()).with_aggregate_id(&aggregate_id_for_blocking);

                let aggregate_event = AggregateEvent {
                    aggregate_id: Some(aggregate_id_for_blocking.clone()),
                    data:         event.clone(),
                    metadata:     Some(metadata)
                };

                all_events.push(aggregate_event);
            }

            let serialized = serde_json::to_vec(&all_events)
                .map_err(|e| WorkflowError::Serialization(format!("Failed to serialize events: {}", e)))?;

            db.put(key.as_bytes(), serialized)
                .map_err(|e| WorkflowError::FileSystem(format!("Failed to write to RocksDB: {}", e)))?;

            Ok(())
        })
        .await
        .map_err(|e| WorkflowError::Generic(format!("Failed to store events: {}", e)))??;

        let new_sequence = self.get_sequence(&aggregate_id).await? + events.len() as u64;
        self.set_sequence(&aggregate_id, new_sequence).await?;

        if new_sequence % self.snapshot_threshold == 0 {
            let current_state = self.rebuild_state(&aggregate_id).await?;
            self.create_snapshot(&aggregate_id, new_sequence, &current_state).await?;
        }

        let mut cache = self.cache.write().await;
        cache.remove(aggregate_id.as_str());

        Ok(())
    }

    /// Get current state for a specific aggregate (session_id)
    async fn get_current_state(&self, aggregate_id: &str) -> Result<WorkflowState, WorkflowError> {
        {
            let cache = self.cache.read().await;
            if let Some(cached_state) = cache.get(aggregate_id) {
                return Ok(cached_state.clone());
            }
        }

        self.rebuild_state(aggregate_id).await
    }

    async fn get_events(&self, session_id: &str) -> Result<Vec<WorkflowEvent>, WorkflowError> {
        let aggregate_events = self.get_events_internal(session_id).await?;
        let workflow_events: Vec<WorkflowEvent> = aggregate_events.iter().map(|ae| ae.data.clone()).collect();
        Ok(workflow_events)
    }
}

/// Factory for creating event stores based on configuration
pub struct EventStoreFactory;

impl EventStoreFactory {
    /// Create an event store based on the specified type
    pub fn create(store_type: EventStoreType, db_path: Option<&Path>) -> Result<Arc<dyn EventStore>, WorkflowError> {
        match store_type {
            EventStoreType::InMemory => Ok(Arc::new(InMemoryEventStore::new())),
            EventStoreType::RocksDb => {
                let path = db_path.ok_or(WorkflowError::Generic(t!("error_rocksdb_path_required")))?;
                Ok(Arc::new(RocksDbEventStore::new(path)?))
            }
        }
    }
}
