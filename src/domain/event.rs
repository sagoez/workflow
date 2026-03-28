//! Event metadata containing common information

use std::{
    collections::HashMap,
    fmt::{self, Debug, Display}
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::workflow::Workflow;

/// Serializable event data for storage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AggregateEvent {
    pub aggregate_id: Option<String>,
    pub data:         WorkflowEvent,
    pub metadata:     Option<EventMetadata>
}

/// Event metadata containing common information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Unique event identifier
    pub event_id:       String,
    /// Event timestamp
    pub timestamp:      DateTime<Utc>,
    /// Event type
    pub event_type:     String,
    /// Aggregate identifier
    pub aggregate_id:   Option<String>,
    /// Correlation identifier
    pub correlation_id: Option<String>,
    /// Causation identifier
    pub causation_id:   Option<String>,
    /// User identifier
    pub user_id:        Option<String>,
    /// Session identifier
    pub session_id:     Option<String>
}

impl EventMetadata {
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_id:       Uuid::new_v4().to_string(),
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

// **********************
// This section contains common events to all workflows

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Workflow discovery event - emitted when workflows are discovered
pub struct WorkflowDiscoveredEvent {
    pub event_id:  String,
    pub timestamp: DateTime<Utc>,
    pub workflow:  Workflow,
    pub file_path: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Workflow selection event - emitted when a workflow is selected for execution
pub struct WorkflowSelectedEvent {
    pub event_id:  String,
    pub timestamp: DateTime<Utc>,
    pub workflow:  Workflow,
    pub user:      String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Workflow started event - emitted when a workflow is started
pub struct WorkflowStartedEvent {
    pub event_id:     String,
    pub timestamp:    DateTime<Utc>,
    pub user:         String,
    pub hostname:     String,
    pub execution_id: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Workflow arguments resolved event - emitted when the arguments for a workflow are resolved
pub struct WorkflowArgumentsResolvedEvent {
    pub event_id:  String,
    pub timestamp: DateTime<Utc>,
    pub arguments: HashMap<String, String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Workflow completed event - emitted when a workflow is completed
pub struct WorkflowCompletedEvent {
    pub event_id:  String,
    pub timestamp: DateTime<Utc>
}

// **********************
// **********************

// This section contains events for specific workflows

/// Unified event enum for all workflow events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowEvent {
    /// Common Events
    WorkflowDiscovered(WorkflowDiscoveredEvent),
    WorkflowSelected(WorkflowSelectedEvent),
    WorkflowStarted(WorkflowStartedEvent),
    WorkflowArgumentsResolved(WorkflowArgumentsResolvedEvent),
    WorkflowCompleted(WorkflowCompletedEvent),

    ///  Command Specific Events
    AvailableWorkflowsListed(AvailableWorkflowsListedEvent),
    SyncRequested(SyncRequestedEvent),
    WorkflowsSynced(WorkflowsSyncedEvent),

    /// Language Management Events
    LanguageSet(LanguageSetEvent),

    /// Storage Management Events
    AggregateReplayed(AggregateReplayedEvent)
}

impl Display for WorkflowEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let event_type = match self {
            WorkflowEvent::WorkflowDiscovered(_) => "WorkflowDiscovered",
            WorkflowEvent::WorkflowSelected(_) => "WorkflowSelected",
            WorkflowEvent::WorkflowStarted(_) => "WorkflowStarted",
            WorkflowEvent::WorkflowArgumentsResolved(_) => "WorkflowArgumentsResolved",
            WorkflowEvent::WorkflowCompleted(_) => "WorkflowCompleted",
            WorkflowEvent::AvailableWorkflowsListed(_) => "AvailableWorkflowsListed",
            WorkflowEvent::SyncRequested(_) => "SyncRequested",
            WorkflowEvent::WorkflowsSynced(_) => "WorkflowsSynced",
            WorkflowEvent::LanguageSet(_) => "LanguageSet",
            WorkflowEvent::AggregateReplayed(_) => "AggregateReplayed"
        };
        write!(f, "{}", event_type)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Available workflows listed event - emitted when the available workflows are listed
pub struct AvailableWorkflowsListedEvent {
    pub event_id:  String,
    pub timestamp: DateTime<Utc>,
    pub workflows: Vec<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Sync requested event - emitted when a sync is requested (intent only)
pub struct SyncRequestedEvent {
    pub event_id:   String,
    pub timestamp:  DateTime<Utc>,
    pub remote_url: String,
    pub branch:     String,
    pub ssh_key:    Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Workflows synced event - emitted when workflows are synced from a git repository
pub struct WorkflowsSyncedEvent {
    pub event_id:     String,
    pub timestamp:    DateTime<Utc>,
    pub remote_url:   String,
    pub branch:       String,
    pub commit_id:    String,
    pub synced_count: u32
}

// **********************
// Language Management Events
// **********************

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Language set event - emitted when the current language is changed
pub struct LanguageSetEvent {
    pub event_id:  String,
    pub timestamp: DateTime<Utc>,
    pub language:  String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Aggregate replayed event - emitted when an aggregate's events are replayed
pub struct AggregateReplayedEvent {
    pub event_id:     String,
    pub timestamp:    DateTime<Utc>,
    pub aggregate_id: String,
    pub events_count: usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_workflow() -> Workflow {
        Workflow {
            name:        "w".to_string(),
            description: "d".to_string(),
            command:     "c".to_string(),
            arguments:   vec![],
            tags:        vec![],
            source_url:  None,
            author:      None,
            author_url:  None,
            shells:      vec![]
        }
    }

    #[test]
    fn workflow_event_display_all_variants() {
        let ts = Utc::now();
        let id = "ev-1".to_string();
        let wf = test_workflow();

        let cases: Vec<(WorkflowEvent, &str)> = vec![
            (
                WorkflowEvent::WorkflowDiscovered(WorkflowDiscoveredEvent {
                    event_id:  id.clone(),
                    timestamp: ts,
                    workflow:  wf.clone(),
                    file_path: "f".to_string()
                }),
                "WorkflowDiscovered"
            ),
            (
                WorkflowEvent::WorkflowSelected(WorkflowSelectedEvent {
                    event_id:  id.clone(),
                    timestamp: ts,
                    workflow:  wf.clone(),
                    user:      "u".to_string()
                }),
                "WorkflowSelected"
            ),
            (
                WorkflowEvent::WorkflowStarted(WorkflowStartedEvent {
                    event_id:     id.clone(),
                    timestamp:    ts,
                    user:         "u".to_string(),
                    hostname:     "h".to_string(),
                    execution_id: "e".to_string()
                }),
                "WorkflowStarted"
            ),
            (
                WorkflowEvent::WorkflowArgumentsResolved(WorkflowArgumentsResolvedEvent {
                    event_id:  id.clone(),
                    timestamp: ts,
                    arguments: HashMap::new()
                }),
                "WorkflowArgumentsResolved"
            ),
            (
                WorkflowEvent::WorkflowCompleted(WorkflowCompletedEvent { event_id: id.clone(), timestamp: ts }),
                "WorkflowCompleted"
            ),
            (
                WorkflowEvent::AvailableWorkflowsListed(AvailableWorkflowsListedEvent {
                    event_id:  id.clone(),
                    timestamp: ts,
                    workflows: vec![]
                }),
                "AvailableWorkflowsListed"
            ),
            (
                WorkflowEvent::SyncRequested(SyncRequestedEvent {
                    event_id:   id.clone(),
                    timestamp:  ts,
                    remote_url: "r".to_string(),
                    branch:     "b".to_string(),
                    ssh_key:    None
                }),
                "SyncRequested"
            ),
            (
                WorkflowEvent::WorkflowsSynced(WorkflowsSyncedEvent {
                    event_id:     id.clone(),
                    timestamp:    ts,
                    remote_url:   "r".to_string(),
                    branch:       "b".to_string(),
                    commit_id:    "c".to_string(),
                    synced_count: 0
                }),
                "WorkflowsSynced"
            ),
            (
                WorkflowEvent::LanguageSet(LanguageSetEvent {
                    event_id:  id.clone(),
                    timestamp: ts,
                    language:  "en".to_string()
                }),
                "LanguageSet"
            ),
            (
                WorkflowEvent::AggregateReplayed(AggregateReplayedEvent {
                    event_id:     id.clone(),
                    timestamp:    ts,
                    aggregate_id: "a".to_string(),
                    events_count: 0
                }),
                "AggregateReplayed"
            ),
        ];

        for (event, expected) in cases {
            assert_eq!(format!("{}", event), expected);
        }
    }

    #[test]
    fn event_metadata_builder() {
        let meta = EventMetadata::new("test-event")
            .with_aggregate_id("agg-1")
            .with_correlation_id("corr-1")
            .with_session_id("sess-1");

        assert_eq!(meta.event_type, "test-event");
        assert_eq!(meta.aggregate_id, Some("agg-1".to_string()));
        assert_eq!(meta.correlation_id, Some("corr-1".to_string()));
        assert_eq!(meta.session_id, Some("sess-1".to_string()));
        assert!(meta.causation_id.is_none());
        assert!(meta.user_id.is_none());
        assert!(!meta.event_id.is_empty());
    }

    #[test]
    fn event_metadata_defaults() {
        let meta = EventMetadata::new("evt");
        assert!(meta.aggregate_id.is_none());
        assert!(meta.correlation_id.is_none());
        assert!(meta.causation_id.is_none());
        assert!(meta.user_id.is_none());
        assert!(meta.session_id.is_none());
    }

    #[test]
    fn aggregate_event_serialization_roundtrip() {
        let ae = AggregateEvent {
            aggregate_id: Some("agg-1".to_string()),
            data:         WorkflowEvent::WorkflowCompleted(WorkflowCompletedEvent {
                event_id:  "ev-1".to_string(),
                timestamp: Utc::now()
            }),
            metadata:     Some(EventMetadata::new("workflow-completed"))
        };

        let json = serde_json::to_string(&ae).unwrap();
        let deserialized: AggregateEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.aggregate_id, Some("agg-1".to_string()));
    }
}
