use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    AppContext,
    domain::{
        command::StartWorkflowCommand,
        engine::EngineContext,
        error::WorkflowError,
        event::{WorkflowEvent, WorkflowStartedEvent},
        state::WorkflowState
    },
    port::command::Command,
    t
};

/// Build a WorkflowStartedEvent from WorkflowSelected state.
/// Returns error if state is not WorkflowSelected.
pub fn build_started_event(
    state: &WorkflowState,
    user: &str,
    hostname: &str
) -> Result<WorkflowStartedEvent, WorkflowError> {
    match state {
        WorkflowState::WorkflowSelected(_) => Ok(WorkflowStartedEvent {
            event_id:     Uuid::new_v4().to_string(),
            timestamp:    Utc::now(),
            user:         user.to_string(),
            hostname:     hostname.to_string(),
            execution_id: Uuid::new_v4().to_string()
        }),
        _ => Err(WorkflowError::Validation(t!("error_no_workflow_selected_to_start")))
    }
}

#[async_trait]
impl Command for StartWorkflowCommand {
    type Error = WorkflowError;
    type LoadedData = ();

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        Ok(())
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        context: &EngineContext,
        _app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        let event =
            build_started_event(current_state, &context.workflow_context.user, &context.workflow_context.hostname)?;
        Ok(vec![WorkflowEvent::WorkflowStarted(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        if let WorkflowState::WorkflowStarted(_) = current_state {
            // nothing to print — prompts follow
        } else {
            eprintln!("{}", t!("error_no_workflow_started"));
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "start-workflow"
    }

    fn description(&self) -> &'static str {
        "Starts the selected workflow"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        state::{InitialState, WorkflowSelectedState},
        workflow::Workflow
    };

    fn test_workflow(name: &str) -> Workflow {
        Workflow {
            name:        name.to_string(),
            description: format!("{} desc", name),
            command:     "echo test".to_string(),
            arguments:   vec![],
            source_url:  None,
            author:      None,
            author_url:  None,
            shells:      vec![],
            tags:        vec![]
        }
    }

    #[test]
    fn build_event_from_selected_state() {
        let state = WorkflowState::WorkflowSelected(WorkflowSelectedState {
            discovered_workflows: vec![test_workflow("deploy")],
            selected_workflow:    test_workflow("deploy")
        });
        let event = build_started_event(&state, "alice", "host1").unwrap();
        assert_eq!(event.user, "alice");
        assert_eq!(event.hostname, "host1");
        assert!(!event.execution_id.is_empty());
    }

    #[test]
    fn build_event_from_wrong_state_returns_error() {
        let state = WorkflowState::Initial(InitialState);
        let result = build_started_event(&state, "alice", "host1");
        assert!(result.is_err());
    }
}
