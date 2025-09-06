use async_trait::async_trait;

use crate::domain::{error::WorkflowError, event::WorkflowEvent, state::WorkflowState};

/// Port for storing and retrieving events
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Store multiple workflow events for a specific session/aggregate
    async fn store_events(&self, session_id: &str, events: &[WorkflowEvent]) -> Result<(), WorkflowError>;

    /// Get current state for a specific aggregate (user/session)
    async fn get_current_state(&self, aggregate_id: &str) -> Result<WorkflowState, WorkflowError>;

    /// Get all events for a specific session/aggregate
    async fn get_events(&self, session_id: &str) -> Result<Vec<WorkflowEvent>, WorkflowError>;
}

/// Port for state restoration from events
#[async_trait]
pub trait StateRestorer: Send + Sync {
    /// Restore state from stored events for a specific aggregate
    async fn restore_state<S: Send + Sync + 'static>(&self, aggregate_id: &str) -> Result<S, WorkflowError>;
}
