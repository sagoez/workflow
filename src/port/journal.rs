use async_trait::async_trait;

use crate::domain::{error::WorkflowError, event::WorkflowEvent};

/// Journal abstraction for event persistence - like Akka Persistence Journal
///
/// The persistence_id is the session_id (business identifier), not actor name!
#[async_trait]
pub trait Journal: Send + Sync {
    /// Persist events for a session (persistence_id = session_id)
    async fn persist_events(&self, session_id: &str, events: &[WorkflowEvent]) -> Result<(), WorkflowError>;

    /// Replay events for recovery (persistence_id = session_id)
    async fn replay_events(&self, session_id: &str, from_sequence: u64) -> Result<Vec<WorkflowEvent>, WorkflowError>;

    /// Get highest sequence number for a session
    async fn highest_sequence_nr(&self, session_id: &str) -> Result<u64, WorkflowError>;

    /// Delete events up to sequence number (for retention)
    async fn delete_events(&self, session_id: &str, to_sequence: u64) -> Result<(), WorkflowError>;
}
