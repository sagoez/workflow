//! In-memory storage implementation for events and state
//!
//! This module provides an in-memory implementation of the EventStore trait
//! for development and testing purposes.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    domain::{
        error::WorkflowError,
        event::{AggregateEvent, EventMetadata, WorkflowEvent},
        state::WorkflowState
    },
    port::{event::Event, storage::EventStore}
};

#[derive(Debug, Clone)]
pub enum EventStoreType {
    InMemory // File
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

        // Apply each event in order to rebuild state using the Event trait
        for aggregate_event in aggregate_events {
            if let Some(new_state) = aggregate_event.data.apply(Some(&current_state)) {
                current_state = new_state;
            } else {
                return Err(WorkflowError::Validation(format!(
                    "Failed to apply event {:?} to state",
                    aggregate_event.data
                )));
            }
        }

        // Cache the rebuilt state
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

        // Convert WorkflowEvents to AggregateEvents and store them
        for event in events {
            let event_type = match event {
                WorkflowEvent::WorkflowDiscovered(_) => "WorkflowDiscovered",
                WorkflowEvent::WorkflowSelected(_) => "WorkflowSelected",
                WorkflowEvent::WorkflowStarted(_) => "WorkflowStarted",
                WorkflowEvent::WorkflowArgumentsResolved(_) => "WorkflowArgumentsResolved",
                WorkflowEvent::WorkflowCompleted(_) => "WorkflowCompleted",
                WorkflowEvent::AvailableWorkflowsListed(_) => "AvailableWorkflowsListed",
                WorkflowEvent::SyncRequested(_) => "SyncRequested",
                WorkflowEvent::WorkflowsSynced(_) => "WorkflowsSynced",
                WorkflowEvent::LanguageSet(_) => "LanguageSet",
                WorkflowEvent::CurrentLanguageRetrieved(_) => "CurrentLanguageRetrieved",
                WorkflowEvent::AvailableLanguagesListed(_) => "AvailableLanguagesListed"
            };

            let metadata = EventMetadata::new(event_type).with_aggregate_id(&aggregate_id);

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
