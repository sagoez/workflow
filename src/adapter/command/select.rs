use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    AppContext,
    domain::{
        command::{InteractivelySelectWorkflowCommand, InteractivelySelectWorkflowData},
        engine::EngineContext,
        error::{ValidationError, WorkflowError},
        event::{WorkflowEvent, WorkflowSelectedEvent},
        state::WorkflowState,
        workflow::Workflow
    },
    port::{command::Command, prompt::UserPrompt},
    t, t_params
};

/// Select a workflow from a list using the UserPrompt trait.
/// Returns the selected Workflow.
pub fn select_workflow(prompt: &dyn UserPrompt, workflows: &[Workflow]) -> Result<Workflow, WorkflowError> {
    let options: Vec<String> = workflows.iter().map(|w| w.name.clone()).collect();

    let selected_name = prompt
        .select(&t!("select_workflow"), options, 10)
        .map_err(|e| ValidationError::SelectionFailed("workflow".to_string(), e.to_string()))?;

    workflows
        .iter()
        .find(|w| w.name == selected_name)
        .cloned()
        .ok_or_else(|| ValidationError::InvalidState(t_params!("error_workflow_not_found", &[&selected_name])).into())
}

#[async_trait]
impl Command for InteractivelySelectWorkflowCommand {
    type Error = WorkflowError;
    type LoadedData = InteractivelySelectWorkflowData;

    async fn load(
        &self,
        _context: &EngineContext,
        app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        if let WorkflowState::WorkflowsDiscovered(state) = current_state {
            let workflow = select_workflow(&*app_context.prompt, &state.discovered_workflows)?;
            Ok(InteractivelySelectWorkflowData { workflow })
        } else {
            Err(ValidationError::InvalidState(t!("error_workflows_not_discovered_yet")).into())
        }
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        loaded_data: &Self::LoadedData,
        context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        let event = WorkflowSelectedEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            workflow:  loaded_data.workflow.clone(),
            user:      context.workflow_context.user.clone()
        };

        Ok(vec![WorkflowEvent::WorkflowSelected(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        if let WorkflowState::WorkflowSelected(_) = current_state {
            // inquire already shows the selection
        } else {
            eprintln!("{}", t!("error_no_workflow_selected"));
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "select-workflow"
    }

    fn description(&self) -> &'static str {
        "Selects and loads a specific workflow"
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
    use crate::adapter::prompt::mock::{MockPrompt, MockPromptResponse};
    use crate::domain::error::PromptError;

    fn test_workflow(name: &str) -> Workflow {
        Workflow {
            name:        name.to_string(),
            description: format!("{} description", name),
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
    fn selects_workflow_by_name() {
        let workflows = vec![test_workflow("deploy"), test_workflow("build"), test_workflow("test")];
        let prompt = MockPrompt::new(vec![MockPromptResponse::Select("build".to_string())]);

        let result = select_workflow(&prompt, &workflows).unwrap();
        assert_eq!(result.name, "build");
    }

    #[test]
    fn returns_error_on_prompt_failure() {
        let workflows = vec![test_workflow("deploy")];
        let prompt =
            MockPrompt::new(vec![MockPromptResponse::Error(PromptError::Interaction("cancelled".to_string()).into())]);

        let result = select_workflow(&prompt, &workflows);
        assert!(result.is_err());
    }

    #[test]
    fn returns_error_when_selected_name_not_found() {
        let workflows = vec![test_workflow("deploy")];
        let prompt = MockPrompt::new(vec![MockPromptResponse::Select("nonexistent".to_string())]);

        let result = select_workflow(&prompt, &workflows);
        assert!(result.is_err());
    }
}
