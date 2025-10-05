//! Argument Resolver - Adapter for interactive user input
//!
//! Handles resolution of workflow arguments through:
//! - Interactive prompts (Select, Text input)
//! - Dynamic command execution for enum values
//! - Custom value entry

use std::collections::HashMap;

use inquire::{Select, Text};
use tokio::process::Command as TokioCommand;

use crate::{
    domain::{error::WorkflowError, workflow::WorkflowArgument},
    t, t_params
};

/// Resolver for workflow arguments - handles user interaction for argument values
pub struct ArgumentResolver;

impl ArgumentResolver {
    /// Resolve all arguments for a workflow
    pub async fn resolve_workflow_arguments(
        arguments: &[WorkflowArgument]
    ) -> Result<HashMap<String, String>, WorkflowError> {
        let mut argument_values = HashMap::new();

        for arg in arguments {
            let value = Self::resolve_argument(arg, &argument_values).await?;
            argument_values.insert(arg.name.clone(), value);
        }

        Ok(argument_values)
    }

    /// Resolve a single workflow argument based on its type
    async fn resolve_argument(
        arg: &WorkflowArgument,
        current_values: &HashMap<String, String>
    ) -> Result<String, WorkflowError> {
        use crate::domain::workflow::ArgumentType;

        match arg.arg_type {
            ArgumentType::Enum => {
                if let Some(enum_variants) = &arg.enum_variants {
                    Self::resolve_static_enum_argument(arg, enum_variants)
                } else if let (Some(enum_command), Some(_enum_name)) = (&arg.enum_command, &arg.enum_name) {
                    Self::resolve_dynamic_enum_argument(arg, enum_command, current_values).await
                } else {
                    Err(WorkflowError::Validation(t_params!("error_enum_argument_missing_configuration", &[&arg.name])))
                }
            }
            ArgumentType::Text | ArgumentType::Number | ArgumentType::Boolean => Self::resolve_simple_argument(arg)
        }
    }

    /// Resolve enum argument with static variants
    fn resolve_static_enum_argument(arg: &WorkflowArgument, variants: &[String]) -> Result<String, WorkflowError> {
        let prompt = t_params!("prompt_select", &[&arg.name]);

        let custom_option = t!("enum_custom_option").to_string();
        let mut options = vec![custom_option.clone()];
        options.extend(variants.iter().cloned());

        let selection = Select::new(&prompt, options).with_page_size(10).prompt().map_err(|e| {
            WorkflowError::Validation(t_params!("error_selection_failed", &[&arg.name, &e.to_string()]))
        })?;

        if selection == custom_option { Self::prompt_for_custom_value(&arg.name) } else { Ok(selection) }
    }

    /// Resolve enum argument with dynamic command execution
    async fn resolve_dynamic_enum_argument(
        arg: &WorkflowArgument,
        enum_command: &str,
        current_values: &HashMap<String, String>
    ) -> Result<String, WorkflowError> {
        let resolved_command = if let Some(ref_arg) = &arg.dynamic_resolution {
            if let Some(ref_value) = current_values.get(ref_arg) {
                enum_command.replace(&format!("{{{{{}}}}}", ref_arg), ref_value)
            } else {
                return Err(WorkflowError::Validation(t_params!("error_dynamic_resolution_failed", &[ref_arg])));
            }
        } else {
            enum_command.to_string()
        };

        println!("{}", t_params!("cli_executing_command_short", &[&resolved_command]));

        let output =
            TokioCommand::new("sh").arg("-c").arg(&resolved_command).output().await.map_err(|e| {
                WorkflowError::Validation(t_params!("error_failed_to_execute_command", &[&e.to_string()]))
            })?;

        if !output.status.success() {
            return Err(WorkflowError::Validation(t_params!(
                "error_command_failed",
                &[&String::from_utf8_lossy(&output.stderr)]
            )));
        }

        let options: Vec<String> = String::from_utf8(output.stdout)
            .map_err(|e| {
                WorkflowError::Validation(t_params!("error_failed_to_parse_command_output", &[&e.to_string()]))
            })?
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if options.is_empty() {
            return Err(WorkflowError::Validation(t_params!("error_no_options_found", &[&arg.name])));
        }

        let prompt = t_params!("prompt_select", &[&arg.name]);

        let custom_option = t!("enum_custom_option").to_string();
        let mut all_options = vec![custom_option.clone()];
        all_options.extend(options);

        let selection = Select::new(&prompt, all_options).with_page_size(10).prompt().map_err(|e| {
            WorkflowError::Validation(t_params!("error_selection_failed", &[&arg.name, &e.to_string()]))
        })?;

        if selection == custom_option { Self::prompt_for_custom_value(&arg.name) } else { Ok(selection) }
    }

    /// Resolve simple text/number/boolean argument
    fn resolve_simple_argument(arg: &WorkflowArgument) -> Result<String, WorkflowError> {
        let prompt = t_params!("prompt_enter", &[&arg.name]);
        let mut text_input = Text::new(&prompt);

        if let Some(default_value) = &arg.default_value
            && !default_value.is_empty()
            && default_value != "~"
        {
            text_input = text_input.with_default(default_value);
        }

        let result = text_input
            .prompt()
            .map_err(|e| WorkflowError::Validation(t_params!("error_input_failed", &[&arg.name, &e.to_string()])))?;

        Ok(result)
    }

    /// Prompt user for a custom value
    fn prompt_for_custom_value(arg_name: &str) -> Result<String, WorkflowError> {
        let custom_prompt = t_params!("enum_enter_custom_value", &[arg_name]);
        let custom_value = Text::new(&custom_prompt)
            .prompt()
            .map_err(|e| WorkflowError::Validation(t_params!("error_input_failed", &[arg_name, &e.to_string()])))?;
        Ok(custom_value)
    }
}

