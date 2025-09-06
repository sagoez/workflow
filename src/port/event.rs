//! Event system for the workflow application
//!
//! This module provides the core event infrastructure including event traits,
//! event data structures, and event handling mechanisms.

use std::fmt::Debug;

use chrono::{DateTime, Utc};

use crate::domain::state::WorkflowState;

/// Core event trait for all domain events
///
/// Events represent state changes in the system and must be:
/// - Pure (no side effects)
/// - Immutable
/// - Serializable
/// - Applied to state to produce new state
pub trait Event: Debug + Send + Sync + 'static {
    /// Apply this event to the current state to produce a new state
    ///
    /// Returns None if the event cannot be applied (indicating an error)
    fn apply(&self, current_state: Option<&WorkflowState>) -> Option<WorkflowState>;

    /// Get the event type identifier
    fn event_type(&self) -> &'static str;

    /// Get the event timestamp
    fn timestamp(&self) -> DateTime<Utc>;

    /// Get the unique event identifier
    fn event_id(&self) -> &str;

    /// Serialize the event to JSON
    fn to_json(&self) -> serde_json::Result<String>;

    /// Get the state type name for this event
    fn state_type(&self) -> &'static str;

    /// Clone this event as a boxed trait object
    fn clone_event(&self) -> Box<dyn Event>;
}
