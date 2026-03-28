use chrono::{DateTime, Utc};

use crate::{
    domain::{
        event::{
            AggregateReplayedEvent, AvailableWorkflowsListedEvent, LanguageSetEvent, SyncRequestedEvent,
            WorkflowArgumentsResolvedEvent, WorkflowCompletedEvent, WorkflowDiscoveredEvent, WorkflowEvent,
            WorkflowSelectedEvent, WorkflowStartedEvent, WorkflowsSyncedEvent
        },
        state::{
            LanguageSetState, SyncRequestedState, WorkflowArgumentsResolvedState, WorkflowCompletedState,
            WorkflowSelectedState, WorkflowStartedState, WorkflowState, WorkflowsDiscoveredState, WorkflowsListedState,
            WorkflowsSyncedState
        }
    },
    port::event::Event
};

// TODO: Move to separate files

impl Event for WorkflowDiscoveredEvent {
    fn apply(&self, current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        let default_state = WorkflowState::default();
        let current = current_state.unwrap_or(&default_state);

        match current {
            WorkflowState::Initial(_) => {
                // First workflow discovered - create new WorkflowsDiscovered state
                Some(WorkflowState::WorkflowsDiscovered(WorkflowsDiscoveredState::new(vec![self.workflow.clone()])))
            }
            WorkflowState::WorkflowsDiscovered(state) => {
                // Add workflow if not already discovered
                let mut workflows = state.discovered_workflows.clone();
                if !workflows.iter().any(|w| w.name == self.workflow.name) {
                    workflows.push(self.workflow.clone());
                }
                Some(WorkflowState::WorkflowsDiscovered(WorkflowsDiscoveredState::new(workflows)))
            }
            _ => None // Invalid transition - can only discover workflows from Initial or already discovered
        }
    }

    fn event_type(&self) -> &'static str {
        "workflow-discovered"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "workflow-state"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

impl Event for WorkflowSelectedEvent {
    fn apply(&self, current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        let default_state = WorkflowState::default();
        let current = current_state.unwrap_or(&default_state);

        match current {
            WorkflowState::WorkflowsDiscovered(state) => {
                // Validate that the selected workflow exists in discovered workflows
                if state.discovered_workflows.iter().any(|w| w.name == self.workflow.name) {
                    Some(WorkflowState::WorkflowSelected(WorkflowSelectedState::new(
                        state.discovered_workflows.clone(),
                        self.workflow.clone()
                    )))
                } else {
                    None // Workflow not found in discovered workflows
                }
            }
            _ => None // Invalid transition - can only select from listed workflows
        }
    }

    fn event_type(&self) -> &'static str {
        "workflow-selected"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "workflow-state"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

impl Event for WorkflowStartedEvent {
    fn apply(&self, current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        let default_state = WorkflowState::default();
        let current = current_state.unwrap_or(&default_state);

        match current {
            WorkflowState::WorkflowSelected(state) => Some(WorkflowState::WorkflowStarted(WorkflowStartedState::new(
                state.discovered_workflows.clone(),
                state.selected_workflow.clone(),
                self.execution_id.clone()
            ))),
            _ => None // Invalid transition - can only start a selected workflow
        }
    }

    fn event_type(&self) -> &'static str {
        "workflow-started"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "workflow-state"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

impl Event for WorkflowArgumentsResolvedEvent {
    fn apply(&self, current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        let default_state = WorkflowState::default();
        let current = current_state.unwrap_or(&default_state);

        match current {
            WorkflowState::WorkflowStarted(state) => {
                Some(WorkflowState::WorkflowArgumentsResolved(WorkflowArgumentsResolvedState::new(
                    state.discovered_workflows.clone(),
                    state.selected_workflow.clone(),
                    state.execution_id.clone(),
                    self.arguments.clone()
                )))
            }
            _ => None // Invalid transition - can only resolve arguments for a started workflow
        }
    }

    fn event_type(&self) -> &'static str {
        "workflow-arguments-resolved"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "workflow-state"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

impl Event for WorkflowCompletedEvent {
    fn apply(&self, current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        let default_state = WorkflowState::default();
        let current = current_state.unwrap_or(&default_state);

        match current {
            WorkflowState::WorkflowArgumentsResolved(state) => {
                Some(WorkflowState::WorkflowCompleted(WorkflowCompletedState::new(
                    state.discovered_workflows.clone(),
                    state.selected_workflow.clone(),
                    state.execution_id.clone(),
                    state.resolved_arguments.clone()
                )))
            }
            _ => None // Invalid transition - can only complete a workflow with resolved arguments
        }
    }

    fn event_type(&self) -> &'static str {
        "workflow-completed"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "workflow-state"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

impl Event for AvailableWorkflowsListedEvent {
    fn apply(&self, current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        let default_state = WorkflowState::default();
        let current = current_state.unwrap_or(&default_state);

        match current {
            WorkflowState::WorkflowsDiscovered(state) => {
                // Transition from discovered to listed
                Some(WorkflowState::WorkflowsListed(WorkflowsListedState::new(state.discovered_workflows.clone())))
            }
            WorkflowState::Initial(_) => {
                // Handle case where no workflows were discovered - transition to listed with empty list
                Some(WorkflowState::WorkflowsListed(WorkflowsListedState::new(vec![])))
            }
            _ => None // Invalid transition
        }
    }

    fn event_type(&self) -> &'static str {
        "available-workflows-listed"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "workflow-state"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

impl Event for WorkflowsSyncedEvent {
    fn apply(&self, current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        match current_state {
            Some(WorkflowState::SyncRequested(_)) => Some(WorkflowState::WorkflowsSynced(WorkflowsSyncedState::new(
                self.remote_url.clone(),
                self.branch.clone(),
                self.commit_id.clone(),
                self.synced_count,
                self.timestamp
            ))),
            _ => None // Invalid state transition
        }
    }

    fn event_type(&self) -> &'static str {
        "workflows-synced"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "workflow-state"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

// **********************
// **********************

impl Event for LanguageSetEvent {
    fn apply(&self, _current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        // Language set can happen from any state - always transitions to LanguageSet
        Some(WorkflowState::LanguageSet(LanguageSetState::new(self.language.clone(), self.timestamp)))
    }

    fn event_type(&self) -> &'static str {
        "language-set"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "workflow-state"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

macro_rules! impl_event {
    ($enum_name:ident { $($variant:ident($field:ident)),* $(,)? }) => {
        impl Event for $enum_name {
            fn apply(&self, current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.apply(current_state),
                    )*
                }
            }

            fn event_type(&self) -> &'static str {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.event_type(),
                    )*
                }
            }

            fn timestamp(&self) -> DateTime<Utc> {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.timestamp(),
                    )*
                }
            }

            fn event_id(&self) -> &str {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.event_id(),
                    )*
                }
            }

            fn to_json(&self) -> serde_json::Result<String> {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.to_json(),
                    )*
                }
            }

            fn state_type(&self) -> &'static str {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.state_type(),
                    )*
                }
            }

            fn clone_event(&self) -> Box<dyn Event> {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.clone_event(),
                    )*
                }
            }
        }
    };
}

impl_event!(WorkflowEvent {
    WorkflowDiscovered(event),
    WorkflowSelected(event),
    WorkflowStarted(event),
    WorkflowArgumentsResolved(event),
    WorkflowCompleted(event),
    AvailableWorkflowsListed(event),
    SyncRequested(event),
    WorkflowsSynced(event),
    LanguageSet(event),
    AggregateReplayed(event)
});

// Individual Event trait implementations
impl Event for SyncRequestedEvent {
    fn apply(&self, _current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        Some(WorkflowState::SyncRequested(SyncRequestedState::new(
            self.remote_url.clone(),
            self.branch.clone(),
            self.ssh_key.clone()
        )))
    }

    fn event_type(&self) -> &'static str {
        "SyncRequested"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "SyncRequestedState"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

impl Event for AggregateReplayedEvent {
    fn apply(&self, current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        current_state.cloned()
    }

    fn event_type(&self) -> &'static str {
        "AggregateReplayed"
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn event_id(&self) -> &str {
        &self.event_id
    }

    fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    fn state_type(&self) -> &'static str {
        "Default"
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        domain::{event::*, state::*, workflow::Workflow},
        port::event::Event
    };

    fn test_workflow() -> Workflow {
        Workflow {
            name:        "test-workflow".to_string(),
            description: "A test workflow".to_string(),
            command:     "echo {{msg}}".to_string(),
            arguments:   vec![],
            source_url:  None,
            author:      None,
            author_url:  None,
            shells:      vec![],
            tags:        vec![]
        }
    }

    fn discovered_state() -> WorkflowState {
        WorkflowState::WorkflowsDiscovered(WorkflowsDiscoveredState::new(vec![test_workflow()]))
    }

    fn selected_state() -> WorkflowState {
        WorkflowState::WorkflowSelected(WorkflowSelectedState::new(vec![test_workflow()], test_workflow()))
    }

    fn started_state() -> WorkflowState {
        WorkflowState::WorkflowStarted(WorkflowStartedState::new(
            vec![test_workflow()],
            test_workflow(),
            "exec-1".to_string()
        ))
    }

    fn resolved_state() -> WorkflowState {
        let mut args = HashMap::new();
        args.insert("msg".to_string(), "hello".to_string());
        WorkflowState::WorkflowArgumentsResolved(WorkflowArgumentsResolvedState::new(
            vec![test_workflow()],
            test_workflow(),
            "exec-1".to_string(),
            args
        ))
    }

    fn sync_requested_state() -> WorkflowState {
        WorkflowState::SyncRequested(SyncRequestedState::new(
            "https://example.com/repo.git".to_string(),
            "main".to_string(),
            None
        ))
    }

    #[test]
    fn discovered_from_initial() {
        let event = WorkflowDiscoveredEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow:  test_workflow(),
            file_path: "test.yaml".to_string()
        };
        let initial = WorkflowState::default();
        let result = event.apply(Some(&initial)).unwrap();
        match result {
            WorkflowState::WorkflowsDiscovered(s) => {
                assert_eq!(s.discovered_workflows.len(), 1);
                assert_eq!(s.discovered_workflows[0].name, "test-workflow");
            }
            _ => panic!("Expected WorkflowsDiscovered")
        }
    }

    #[test]
    fn discovered_adds_to_existing() {
        let event = WorkflowDiscoveredEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow:  Workflow { name: "second-workflow".to_string(), ..test_workflow() },
            file_path: "second.yaml".to_string()
        };
        let result = event.apply(Some(&discovered_state())).unwrap();
        match result {
            WorkflowState::WorkflowsDiscovered(s) => assert_eq!(s.discovered_workflows.len(), 2),
            _ => panic!("Expected WorkflowsDiscovered")
        }
    }

    #[test]
    fn discovered_deduplicates_by_name() {
        let event = WorkflowDiscoveredEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow:  test_workflow(),
            file_path: "test.yaml".to_string()
        };
        let result = event.apply(Some(&discovered_state())).unwrap();
        match result {
            WorkflowState::WorkflowsDiscovered(s) => assert_eq!(s.discovered_workflows.len(), 1),
            _ => panic!("Expected WorkflowsDiscovered")
        }
    }

    #[test]
    fn discovered_from_invalid_state_returns_none() {
        let event = WorkflowDiscoveredEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow:  test_workflow(),
            file_path: "test.yaml".to_string()
        };
        assert!(event.apply(Some(&selected_state())).is_none());
    }

    #[test]
    fn discovered_from_none_uses_default() {
        let event = WorkflowDiscoveredEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow:  test_workflow(),
            file_path: "test.yaml".to_string()
        };
        let result = event.apply(None).unwrap();
        match result {
            WorkflowState::WorkflowsDiscovered(s) => assert_eq!(s.discovered_workflows.len(), 1),
            _ => panic!("Expected WorkflowsDiscovered")
        }
    }

    #[test]
    fn selected_from_discovered() {
        let event = WorkflowSelectedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow:  test_workflow(),
            user:      "tester".to_string()
        };
        let result = event.apply(Some(&discovered_state())).unwrap();
        match result {
            WorkflowState::WorkflowSelected(s) => assert_eq!(s.selected_workflow.name, "test-workflow"),
            _ => panic!("Expected WorkflowSelected")
        }
    }

    #[test]
    fn selected_unknown_workflow_returns_none() {
        let event = WorkflowSelectedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow:  Workflow { name: "nonexistent".to_string(), ..test_workflow() },
            user:      "tester".to_string()
        };
        assert!(event.apply(Some(&discovered_state())).is_none());
    }

    #[test]
    fn selected_from_invalid_state_returns_none() {
        let event = WorkflowSelectedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow:  test_workflow(),
            user:      "tester".to_string()
        };
        assert!(event.apply(Some(&WorkflowState::default())).is_none());
    }

    #[test]
    fn started_from_selected() {
        let event = WorkflowStartedEvent {
            event_id:     Uuid::new_v4().to_string(),
            timestamp:    Utc::now(),
            user:         "tester".to_string(),
            hostname:     "host".to_string(),
            execution_id: "exec-42".to_string()
        };
        let result = event.apply(Some(&selected_state())).unwrap();
        match result {
            WorkflowState::WorkflowStarted(s) => {
                assert_eq!(s.execution_id, "exec-42");
                assert_eq!(s.selected_workflow.name, "test-workflow");
            }
            _ => panic!("Expected WorkflowStarted")
        }
    }

    #[test]
    fn started_from_invalid_state_returns_none() {
        let event = WorkflowStartedEvent {
            event_id:     Uuid::new_v4().to_string(),
            timestamp:    Utc::now(),
            user:         "tester".to_string(),
            hostname:     "host".to_string(),
            execution_id: "exec-42".to_string()
        };
        assert!(event.apply(Some(&discovered_state())).is_none());
    }

    #[test]
    fn arguments_resolved_from_started() {
        let mut args = HashMap::new();
        args.insert("msg".to_string(), "hello".to_string());
        let event = WorkflowArgumentsResolvedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            arguments: args.clone()
        };
        let result = event.apply(Some(&started_state())).unwrap();
        match result {
            WorkflowState::WorkflowArgumentsResolved(s) => {
                assert_eq!(s.resolved_arguments, args);
                assert_eq!(s.execution_id, "exec-1");
            }
            _ => panic!("Expected WorkflowArgumentsResolved")
        }
    }

    #[test]
    fn arguments_resolved_from_invalid_state_returns_none() {
        let event = WorkflowArgumentsResolvedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            arguments: HashMap::new()
        };
        assert!(event.apply(Some(&selected_state())).is_none());
    }

    #[test]
    fn completed_from_resolved() {
        let event = WorkflowCompletedEvent { event_id: Uuid::new_v4().to_string(), timestamp: Utc::now() };
        let result = event.apply(Some(&resolved_state())).unwrap();
        match result {
            WorkflowState::WorkflowCompleted(s) => {
                assert_eq!(s.completed_workflow.name, "test-workflow");
                assert_eq!(s.execution_id, "exec-1");
                assert_eq!(s.resolved_arguments.get("msg").unwrap(), "hello");
            }
            _ => panic!("Expected WorkflowCompleted")
        }
    }

    #[test]
    fn completed_from_invalid_state_returns_none() {
        let event = WorkflowCompletedEvent { event_id: Uuid::new_v4().to_string(), timestamp: Utc::now() };
        assert!(event.apply(Some(&started_state())).is_none());
    }

    #[test]
    fn listed_from_discovered() {
        let event = AvailableWorkflowsListedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflows: vec!["test-workflow".to_string()]
        };
        let result = event.apply(Some(&discovered_state())).unwrap();
        match result {
            WorkflowState::WorkflowsListed(s) => assert_eq!(s.discovered_workflows.len(), 1),
            _ => panic!("Expected WorkflowsListed")
        }
    }

    #[test]
    fn listed_from_initial_gives_empty() {
        let event = AvailableWorkflowsListedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflows: vec![]
        };
        let result = event.apply(Some(&WorkflowState::default())).unwrap();
        match result {
            WorkflowState::WorkflowsListed(s) => assert!(s.discovered_workflows.is_empty()),
            _ => panic!("Expected WorkflowsListed")
        }
    }

    #[test]
    fn listed_from_invalid_state_returns_none() {
        let event = AvailableWorkflowsListedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflows: vec![]
        };
        assert!(event.apply(Some(&selected_state())).is_none());
    }

    #[test]
    fn sync_requested_from_any_state() {
        let event = SyncRequestedEvent {
            event_id:   Uuid::new_v4().to_string(),
            timestamp:  Utc::now(),
            remote_url: "https://example.com/repo.git".to_string(),
            branch:     "main".to_string(),
            ssh_key:    Some("/path/to/key".to_string())
        };
        let result = event.apply(Some(&WorkflowState::default())).unwrap();
        match result {
            WorkflowState::SyncRequested(s) => {
                assert_eq!(s.remote_url, "https://example.com/repo.git");
                assert_eq!(s.ssh_key, Some("/path/to/key".to_string()));
            }
            _ => panic!("Expected SyncRequested")
        }
    }

    #[test]
    fn synced_from_sync_requested() {
        let event = WorkflowsSyncedEvent {
            event_id:     Uuid::new_v4().to_string(),
            timestamp:    Utc::now(),
            remote_url:   "https://example.com/repo.git".to_string(),
            branch:       "main".to_string(),
            commit_id:    "abc123".to_string(),
            synced_count: 5
        };
        let result = event.apply(Some(&sync_requested_state())).unwrap();
        match result {
            WorkflowState::WorkflowsSynced(s) => {
                assert_eq!(s.commit_id, "abc123");
                assert_eq!(s.synced_count, 5);
            }
            _ => panic!("Expected WorkflowsSynced")
        }
    }

    #[test]
    fn synced_from_invalid_state_returns_none() {
        let event = WorkflowsSyncedEvent {
            event_id:     Uuid::new_v4().to_string(),
            timestamp:    Utc::now(),
            remote_url:   "r".to_string(),
            branch:       "b".to_string(),
            commit_id:    "c".to_string(),
            synced_count: 0
        };
        assert!(event.apply(Some(&WorkflowState::default())).is_none());
    }

    #[test]
    fn language_set_from_any_state() {
        let event = LanguageSetEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            language:  "es".to_string()
        };
        let result = event.apply(Some(&discovered_state())).unwrap();
        match result {
            WorkflowState::LanguageSet(s) => assert_eq!(s.language, "es"),
            _ => panic!("Expected LanguageSet")
        }
    }

    #[test]
    fn aggregate_replayed_preserves_state() {
        let event = AggregateReplayedEvent {
            event_id:     Uuid::new_v4().to_string(),
            timestamp:    Utc::now(),
            aggregate_id: "agg-1".to_string(),
            events_count: 10
        };
        let result = event.apply(Some(&discovered_state())).unwrap();
        match result {
            WorkflowState::WorkflowsDiscovered(s) => assert_eq!(s.discovered_workflows.len(), 1),
            _ => panic!("Expected state to be preserved")
        }
    }

    #[test]
    fn aggregate_replayed_with_none_returns_none() {
        let event = AggregateReplayedEvent {
            event_id:     Uuid::new_v4().to_string(),
            timestamp:    Utc::now(),
            aggregate_id: "agg-1".to_string(),
            events_count: 0
        };
        assert!(event.apply(None).is_none());
    }

    #[test]
    fn event_types_are_correct() {
        let ts = Utc::now();
        let id = Uuid::new_v4().to_string();

        assert_eq!(
            WorkflowDiscoveredEvent {
                event_id:  id.clone(),
                timestamp: ts,
                workflow:  test_workflow(),
                file_path: "f".to_string()
            }
            .event_type(),
            "workflow-discovered"
        );
        assert_eq!(
            WorkflowSelectedEvent {
                event_id:  id.clone(),
                timestamp: ts,
                workflow:  test_workflow(),
                user:      "u".to_string()
            }
            .event_type(),
            "workflow-selected"
        );
        assert_eq!(
            WorkflowStartedEvent {
                event_id:     id.clone(),
                timestamp:    ts,
                user:         "u".to_string(),
                hostname:     "h".to_string(),
                execution_id: "e".to_string()
            }
            .event_type(),
            "workflow-started"
        );
        assert_eq!(
            WorkflowArgumentsResolvedEvent { event_id: id.clone(), timestamp: ts, arguments: HashMap::new() }
                .event_type(),
            "workflow-arguments-resolved"
        );
        assert_eq!(WorkflowCompletedEvent { event_id: id.clone(), timestamp: ts }.event_type(), "workflow-completed");
        assert_eq!(
            AvailableWorkflowsListedEvent { event_id: id.clone(), timestamp: ts, workflows: vec![] }.event_type(),
            "available-workflows-listed"
        );
        assert_eq!(
            SyncRequestedEvent {
                event_id:   id.clone(),
                timestamp:  ts,
                remote_url: "r".to_string(),
                branch:     "b".to_string(),
                ssh_key:    None
            }
            .event_type(),
            "SyncRequested"
        );
        assert_eq!(
            WorkflowsSyncedEvent {
                event_id:     id.clone(),
                timestamp:    ts,
                remote_url:   "r".to_string(),
                branch:       "b".to_string(),
                commit_id:    "c".to_string(),
                synced_count: 0
            }
            .event_type(),
            "workflows-synced"
        );
        assert_eq!(
            LanguageSetEvent { event_id: id.clone(), timestamp: ts, language: "en".to_string() }.event_type(),
            "language-set"
        );
        assert_eq!(
            AggregateReplayedEvent {
                event_id:     id.clone(),
                timestamp:    ts,
                aggregate_id: "a".to_string(),
                events_count: 0
            }
            .event_type(),
            "AggregateReplayed"
        );
    }

    #[test]
    fn events_serialize_to_json() {
        let event = WorkflowDiscoveredEvent {
            event_id:  "ev-1".to_string(),
            timestamp: Utc::now(),
            workflow:  test_workflow(),
            file_path: "test.yaml".to_string()
        };
        let json = event.to_json().unwrap();
        assert!(json.contains("ev-1"));
        assert!(json.contains("test-workflow"));
    }
}
