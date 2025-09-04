//! Event system for the workflow application
//!
//! This module provides the core event infrastructure including event traits,
//! event data structures, and event handling mechanisms.

use std::{any::Any, fmt::Debug};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    /// The state is passed and returned as Any to allow dynamic typing
    fn apply(&self, current_state: Option<&dyn Any>) -> Option<Box<dyn Any>>;

    /// Get the event type identifier
    fn event_type(&self) -> &'static str;

    /// Get the event timestamp
    fn timestamp(&self) -> DateTime<Utc>;

    /// Get the unique event identifier
    fn event_id(&self) -> &str;

    /// Get the aggregate identifier (optional)
    fn aggregate_id(&self) -> Option<&str> {
        None
    }

    /// Get the correlation identifier (optional)
    fn correlation_id(&self) -> Option<&str> {
        None
    }

    /// Serialize the event to JSON
    fn to_json(&self) -> serde_json::Result<String>;

    /// Get the state type name for this event
    fn state_type(&self) -> &'static str;

    /// Clone this event as a boxed trait object
    fn clone_event(&self) -> Box<dyn Event>;
}

/// Event metadata containing common information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub event_id:       String,
    pub timestamp:      DateTime<Utc>,
    pub event_type:     String,
    pub aggregate_id:   Option<String>,
    pub correlation_id: Option<String>,
    pub causation_id:   Option<String>,
    pub user_id:        Option<String>,
    pub session_id:     Option<String>
}

impl EventMetadata {
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_id:       uuid::Uuid::new_v4().to_string(),
            timestamp:      Utc::now(),
            event_type:     event_type.into(),
            aggregate_id:   None,
            correlation_id: None,
            causation_id:   None,
            user_id:        None,
            session_id:     None
        }
    }

    pub fn with_aggregate_id(mut self, aggregate_id: impl Into<String>) -> Self {
        self.aggregate_id = Some(aggregate_id.into());
        self
    }

    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// Event processing result
#[derive(Debug, Clone)]
pub enum EventResult {
    Success,
    Failed(String),
    Skipped(String)
}
