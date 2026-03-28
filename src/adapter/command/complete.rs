use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    AppContext,
    domain::{
        command::CompleteWorkflowCommand,
        engine::EngineContext,
        error::WorkflowError,
        event::{WorkflowCompletedEvent, WorkflowEvent},
        state::WorkflowState
    },
    port::command::Command,
    t
};

/// Build a WorkflowCompletedEvent from WorkflowArgumentsResolved state.
/// Returns error if state is not WorkflowArgumentsResolved.
pub fn build_completed_event(state: &WorkflowState) -> Result<WorkflowCompletedEvent, WorkflowError> {
    match state {
        WorkflowState::WorkflowArgumentsResolved(_) => {
            Ok(WorkflowCompletedEvent {
                event_id:  Uuid::new_v4().to_string(),
                timestamp: Utc::now()
            })
        }
        _ => Err(WorkflowError::Validation(t!("error_no_workflow_ready_to_complete")))
    }
}

#[async_trait]
impl Command for CompleteWorkflowCommand {
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
        _context: &EngineContext,
        _app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        let event = build_completed_event(current_state)?;
        Ok(vec![WorkflowEvent::WorkflowCompleted(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        if let WorkflowState::WorkflowCompleted(_) = current_state {
            // nothing to print — command was already shown
        } else {
            eprintln!("{}", t!("error_no_workflow_completed"));
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "complete-workflow"
    }

    fn description(&self) -> &'static str {
        "Marks the current workflow as completed"
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
    use std::collections::HashMap;

    use super::*;
    use crate::domain::{
        state::{InitialState, WorkflowArgumentsResolvedState},
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
    fn build_event_from_resolved_state() {
        let state = WorkflowState::WorkflowArgumentsResolved(WorkflowArgumentsResolvedState {
            discovered_workflows: vec![test_workflow("deploy")],
            selected_workflow:    test_workflow("deploy"),
            execution_id:         "exec-1".to_string(),
            resolved_arguments:   HashMap::new()
        });
        let event = build_completed_event(&state).unwrap();
        assert!(!event.event_id.is_empty());
    }

    #[test]
    fn build_event_from_wrong_state_returns_error() {
        let state = WorkflowState::Initial(InitialState);
        let result = build_completed_event(&state);
        assert!(result.is_err());
    }
}
