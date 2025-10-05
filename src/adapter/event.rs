use chrono::{DateTime, Utc};

use crate::{
    domain::{
        event::{
            AggregateReplayedEvent, AggregatesListedEvent, AvailableLanguagesListedEvent,
            AvailableWorkflowsListedEvent, CurrentLanguageRetrievedEvent, LanguageSetEvent, SyncRequestedEvent,
            WorkflowArgumentsResolvedEvent, WorkflowCompletedEvent, WorkflowDiscoveredEvent, WorkflowEvent,
            WorkflowSelectedEvent, WorkflowStartedEvent, WorkflowsSyncedEvent
        },
        state::{
            AvailableLanguagesListedState, CurrentLanguageRetrievedState, LanguageSetState, SyncRequestedState,
            WorkflowArgumentsResolvedState, WorkflowCompletedState, WorkflowSelectedState, WorkflowStartedState,
            WorkflowState, WorkflowsDiscoveredState, WorkflowsListedState, WorkflowsSyncedState
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
                Some(WorkflowState::WorkflowsDiscovered(WorkflowsDiscoveredState {
                    discovered_workflows: vec![self.workflow.clone()]
                }))
            }
            WorkflowState::WorkflowsDiscovered(state) => {
                // Add workflow if not already discovered
                let mut workflows = state.discovered_workflows.clone();
                if !workflows.iter().any(|w| w.name == self.workflow.name) {
                    workflows.push(self.workflow.clone());
                }
                Some(WorkflowState::WorkflowsDiscovered(WorkflowsDiscoveredState { discovered_workflows: workflows }))
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
                    Some(WorkflowState::WorkflowSelected(WorkflowSelectedState {
                        discovered_workflows: state.discovered_workflows.clone(),
                        selected_workflow:    self.workflow.clone()
                    }))
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
            WorkflowState::WorkflowSelected(state) => Some(WorkflowState::WorkflowStarted(WorkflowStartedState {
                discovered_workflows: state.discovered_workflows.clone(),
                selected_workflow:    state.selected_workflow.clone(),
                execution_id:         self.execution_id.clone()
            })),
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
                Some(WorkflowState::WorkflowArgumentsResolved(WorkflowArgumentsResolvedState {
                    discovered_workflows: state.discovered_workflows.clone(),
                    selected_workflow:    state.selected_workflow.clone(),
                    execution_id:         state.execution_id.clone(),
                    resolved_arguments:   self.arguments.clone()
                }))
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
                Some(WorkflowState::WorkflowCompleted(WorkflowCompletedState {
                    discovered_workflows: state.discovered_workflows.clone(),
                    completed_workflow:   state.selected_workflow.clone(),
                    execution_id:         state.execution_id.clone(),
                    resolved_arguments:   state.resolved_arguments.clone()
                }))
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
                Some(WorkflowState::WorkflowsListed(WorkflowsListedState {
                    discovered_workflows: state.discovered_workflows.clone()
                }))
            }
            WorkflowState::Initial(_) => {
                // Handle case where no workflows were discovered - transition to listed with empty list
                Some(WorkflowState::WorkflowsListed(WorkflowsListedState { discovered_workflows: vec![] }))
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
            Some(WorkflowState::SyncRequested(_)) => Some(WorkflowState::WorkflowsSynced(WorkflowsSyncedState {
                remote_url:   self.remote_url.clone(),
                branch:       self.branch.clone(),
                commit_id:    self.commit_id.clone(),
                synced_count: self.synced_count,
                synced_at:    self.timestamp
            })),
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
        Some(WorkflowState::LanguageSet(LanguageSetState { language: self.language.clone(), set_at: self.timestamp }))
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

impl Event for CurrentLanguageRetrievedEvent {
    fn apply(&self, _current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        // Language retrieval can happen from any state - always transitions to CurrentLanguageRetrieved
        Some(WorkflowState::CurrentLanguageRetrieved(CurrentLanguageRetrievedState {
            language:     self.language.clone(),
            retrieved_at: self.timestamp
        }))
    }

    fn event_type(&self) -> &'static str {
        "current-language-retrieved"
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

impl Event for AvailableLanguagesListedEvent {
    fn apply(&self, _current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        // Language listing can happen from any state - always transitions to AvailableLanguagesListed
        Some(WorkflowState::AvailableLanguagesListed(AvailableLanguagesListedState {
            languages: self.languages.clone(),
            listed_at: self.timestamp
        }))
    }

    fn event_type(&self) -> &'static str {
        "available-languages-listed"
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
    CurrentLanguageRetrieved(event),
    AvailableLanguagesListed(event),
    AggregatesListed(event),
    AggregateReplayed(event)
});

// Individual Event trait implementations
impl Event for SyncRequestedEvent {
    fn apply(&self, _current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        Some(WorkflowState::SyncRequested(SyncRequestedState {
            remote_url: self.remote_url.clone(),
            branch:     self.branch.clone(),
            ssh_key:    self.ssh_key.clone()
        }))
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

impl Event for AggregatesListedEvent {
    fn apply(&self, current_state: Option<&WorkflowState>) -> Option<WorkflowState> {
        current_state.cloned()
    }

    fn event_type(&self) -> &'static str {
        "AggregatesListed"
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
