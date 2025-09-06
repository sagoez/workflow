//! Journal Implementations - Pluggable Event Persistence
//!
//! Like Akka Persistence, you can switch storage backends by configuration:
//! - InMemoryJournal: For development/testing
//! - CassandraJournal: For production (TODO)
//! - PostgreSQLJournal: Alternative production option (TODO)

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    domain::{error::WorkflowError, event::WorkflowEvent},
    port::journal::Journal
};

/// In-Memory Journal Implementation
///
/// Simple HashMap-based storage for development and testing.
/// Events are stored by session_id (persistence_id).
pub struct InMemoryJournal {
    /// Events stored by session_id -> Vec<WorkflowEvent>
    events: Arc<RwLock<HashMap<String, Vec<WorkflowEvent>>>>
}

impl InMemoryJournal {
    pub fn new() -> Self {
        Self { events: Arc::new(RwLock::new(HashMap::new())) }
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
            session_events.push(event.clone());
        }

        Ok(())
    }

    async fn replay_events(&self, session_id: &str, from_sequence: u64) -> Result<Vec<WorkflowEvent>, WorkflowError> {
        let store = self.events.read().await;

        if let Some(session_events) = store.get(session_id) {
            let events = session_events.iter().skip(from_sequence as usize).cloned().collect();
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

/// Journal Factory - Configuration-driven journal creation
pub enum JournalType {
    InMemory // TODO: Add Cassandra, PostgreSQL, etc.
}

pub struct JournalFactory;

impl JournalFactory {
    /// Create journal based on configuration
    pub fn create(journal_type: JournalType) -> Arc<dyn Journal> {
        match journal_type {
            JournalType::InMemory => Arc::new(InMemoryJournal::new())
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
