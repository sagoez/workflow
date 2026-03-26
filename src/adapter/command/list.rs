use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    AppContext,
    domain::{
        command::ListWorkflowsCommand,
        engine::EngineContext,
        error::WorkflowError,
        event::{AvailableWorkflowsListedEvent, WorkflowEvent},
        state::WorkflowState
    },
    port::command::Command,
    t
};

/// Extract workflow names from current state.
/// Returns names from WorkflowsDiscovered, empty vec from Initial, error otherwise.
pub fn list_workflow_names(state: &WorkflowState) -> Result<Vec<String>, WorkflowError> {
    match state {
        WorkflowState::WorkflowsDiscovered(s) => {
            Ok(s.discovered_workflows.iter().map(|w| w.name.clone()).collect())
        }
        WorkflowState::Initial(_) => Ok(vec![]),
        _ => Err(WorkflowError::Validation(t!("error_workflows_not_discovered_yet")))
    }
}

#[async_trait]
impl Command for ListWorkflowsCommand {
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
        let workflow_names = list_workflow_names(current_state)?;
        let event = AvailableWorkflowsListedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            workflows: workflow_names
        };

        Ok(vec![WorkflowEvent::AvailableWorkflowsListed(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        println!("{}", t!("cli_available_workflows"));
        println!();
        match current_state {
            WorkflowState::WorkflowsListed(state) => {
                for workflow in &state.discovered_workflows {
                    println!("  - {}", workflow.name);
                }
                if state.discovered_workflows.is_empty() {
                    println!("  {}", t!("no_workflows_found"));
                }
            }
            _ => {
                println!("  {}", t!("no_workflows_found"));
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "list-workflows"
    }

    fn description(&self) -> &'static str {
        "Lists all available workflow YAML files"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        state::{InitialState, WorkflowSelectedState, WorkflowsDiscoveredState},
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
    fn list_names_from_discovered_state() {
        let state = WorkflowState::WorkflowsDiscovered(WorkflowsDiscoveredState {
            discovered_workflows: vec![test_workflow("deploy"), test_workflow("build")]
        });
        let names = list_workflow_names(&state).unwrap();
        assert_eq!(names, vec!["deploy", "build"]);
    }

    #[test]
    fn list_names_from_initial_state_returns_empty() {
        let state = WorkflowState::Initial(InitialState);
        let names = list_workflow_names(&state).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn list_names_from_wrong_state_returns_error() {
        let state = WorkflowState::WorkflowSelected(WorkflowSelectedState {
            discovered_workflows: vec![test_workflow("deploy")],
            selected_workflow:    test_workflow("deploy")
        });
        let result = list_workflow_names(&state);
        assert!(result.is_err());
    }
}
