use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    AppContext,
    adapter::resolver::ArgumentResolver,
    domain::{
        command::{ResolveArgumentsCommand, ResolveArgumentsData},
        engine::EngineContext,
        error::WorkflowError,
        event::{WorkflowArgumentsResolvedEvent, WorkflowEvent},
        state::WorkflowState,
        workflow::WorkflowArgument
    },
    port::command::Command,
    t, t_params
};

/// Validate that all workflow arguments have been resolved.
/// Returns Ok(()) if every argument name has a corresponding entry in resolved_arguments.
pub fn validate_all_resolved(
    arguments: &[WorkflowArgument],
    resolved: &HashMap<String, String>
) -> Result<(), WorkflowError> {
    for arg in arguments {
        if !resolved.contains_key(&arg.name) {
            return Err(WorkflowError::Validation(t_params!("error_argument_not_resolved", &[&arg.name])));
        }
    }
    Ok(())
}

/// Render a command template with resolved arguments using Tera.
/// Replaces `{{ var }}` placeholders with their values.
pub fn render_command_template(
    template: &str,
    resolved: &HashMap<String, String>
) -> Result<String, WorkflowError> {
    let mut tera = tera::Tera::default();
    let mut context = tera::Context::new();

    for (key, value) in resolved {
        context.insert(key, value);
    }

    tera.render_str(template, &context).map_err(|e| {
        WorkflowError::Validation(t_params!("error_failed_to_render_command_template", &[&e.to_string()]))
    })
}

#[async_trait]
impl Command for ResolveArgumentsCommand {
    type Error = WorkflowError;
    type LoadedData = ResolveArgumentsData;

    async fn load(
        &self,
        _context: &EngineContext,
        app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        let workflow = match current_state {
            WorkflowState::WorkflowStarted(state) => state.selected_workflow.clone(),
            _ => return Err(WorkflowError::Validation(t!("error_no_workflow_started_to_resolve_arguments")))
        };

        let resolved_arguments =
            ArgumentResolver::resolve_workflow_arguments(
                &workflow.arguments,
                &*app_context.prompt,
                &*app_context.executor
            ).await.map_err(|e| {
                WorkflowError::Validation(t_params!("error_failed_to_resolve_arguments", &[&e.to_string()]))
            })?;

        Ok(ResolveArgumentsData { workflow, resolved_arguments })
    }

    fn validate(&self, loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        validate_all_resolved(&loaded_data.workflow.arguments, &loaded_data.resolved_arguments)
    }

    async fn emit(
        &self,
        loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        match current_state {
            WorkflowState::WorkflowStarted(_) => {
                let event = WorkflowArgumentsResolvedEvent {
                    event_id:  Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    arguments: loaded_data.resolved_arguments.clone()
                };

                Ok(vec![WorkflowEvent::WorkflowArgumentsResolved(event)])
            }
            _ => Err(WorkflowError::Validation(t!("error_no_workflow_execution_in_progress")))
        }
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        match current_state {
            WorkflowState::WorkflowArgumentsResolved(state) => {
                let workflow = &state.selected_workflow;
                println!("{}", t_params!("cli_resolved_arguments_for_workflow", &[&workflow.name]));

                for (key, value) in &state.resolved_arguments {
                    println!("  {} = {}", key, value);
                }

                let rendered_command = render_command_template(&workflow.command, &state.resolved_arguments)?;

                println!("{}", t!("cli_generated_command"));
                println!("{}", rendered_command);

                match super::copy_to_clipboard(&rendered_command) {
                    Ok(()) => {
                        println!("{}", t!("cli_command_copied_to_clipboard"));
                        println!("{}", t!("cli_command_can_now_be_pasted_and_executed_in_terminal"));
                    }
                    Err(e) => {
                        println!("{}", t_params!("cli_failed_to_copy_to_clipboard", &[&e.to_string()]));
                        println!("{}", t!("cli_command_can_now_be_pasted_and_executed_in_terminal"));
                    }
                }
            }
            _ => {
                println!("{}", t!("error_no_arguments_resolved"));
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "resolve-arguments"
    }

    fn description(&self) -> &'static str {
        "Interactively resolves workflow arguments with dynamic resolution"
    }

    fn is_interactive(&self) -> bool {
        true
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::workflow::{ArgumentType, WorkflowArgument};

    fn text_arg(name: &str) -> WorkflowArgument {
        WorkflowArgument {
            name:               name.to_string(),
            arg_type:           ArgumentType::Text,
            description:        format!("{} arg", name),
            default_value:      None,
            enum_name:          None,
            enum_command:       None,
            enum_variants:      None,
            dynamic_resolution: None,
            min_selections:     None,
            max_selections:     None
        }
    }

    #[test]
    fn validate_passes_when_all_resolved() {
        let args = vec![text_arg("env"), text_arg("region")];
        let mut resolved = HashMap::new();
        resolved.insert("env".to_string(), "prod".to_string());
        resolved.insert("region".to_string(), "us-east-1".to_string());

        assert!(validate_all_resolved(&args, &resolved).is_ok());
    }

    #[test]
    fn validate_fails_when_argument_missing() {
        let args = vec![text_arg("env"), text_arg("region")];
        let mut resolved = HashMap::new();
        resolved.insert("env".to_string(), "prod".to_string());

        let result = validate_all_resolved(&args, &resolved);
        assert!(result.is_err());
    }

    #[test]
    fn render_template_replaces_placeholders() {
        let mut resolved = HashMap::new();
        resolved.insert("name".to_string(), "world".to_string());
        resolved.insert("count".to_string(), "3".to_string());

        let result = render_command_template("echo {{ name }} {{ count }}", &resolved).unwrap();
        assert_eq!(result, "echo world 3");
    }

    #[test]
    fn render_template_error_on_missing_variable() {
        let resolved = HashMap::new();
        let result = render_command_template("echo {{ missing }}", &resolved);
        assert!(result.is_err());
    }
}
