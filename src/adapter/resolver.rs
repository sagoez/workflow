//! Argument Resolver - Adapter for interactive user input
//!
//! Handles resolution of workflow arguments through:
//! - Interactive prompts (Select, Text input)
//! - Dynamic command execution for enum values
//! - Custom value entry

use std::collections::HashMap;

const PAGE_SIZE: usize = 10;

use crate::{
    domain::{
        error::{ValidationError, WorkflowError},
        workflow::WorkflowArgument
    },
    port::{executor::CommandExecutor, prompt::UserPrompt},
    t, t_params
};

/// Resolver for workflow arguments - handles user interaction for argument values
pub struct ArgumentResolver;

impl ArgumentResolver {
    /// Resolve all arguments for a workflow
    pub async fn resolve_workflow_arguments(
        arguments: &[WorkflowArgument],
        prompt: &dyn UserPrompt,
        executor: &dyn CommandExecutor
    ) -> Result<HashMap<String, String>, WorkflowError> {
        let mut argument_values = HashMap::new();

        for arg in arguments {
            let value = Self::resolve_argument(arg, &argument_values, prompt, executor).await?;
            argument_values.insert(arg.name.clone(), value);
        }

        Ok(argument_values)
    }

    /// Resolve a single workflow argument based on its type
    async fn resolve_argument(
        arg: &WorkflowArgument,
        current_values: &HashMap<String, String>,
        prompt: &dyn UserPrompt,
        executor: &dyn CommandExecutor
    ) -> Result<String, WorkflowError> {
        use crate::domain::workflow::ArgumentType;

        match arg.arg_type {
            ArgumentType::Enum => {
                let multi = arg.multi;

                if let Some(enum_variants) = &arg.enum_variants {
                    if multi {
                        Self::resolve_static_multi_enum_argument(arg, enum_variants, prompt)
                    } else {
                        Self::resolve_static_enum_argument(arg, enum_variants, prompt)
                    }
                } else if let (Some(enum_command), Some(_enum_name)) = (&arg.enum_command, &arg.enum_name) {
                    if multi {
                        Self::resolve_dynamic_multi_enum_argument(arg, enum_command, current_values, prompt, executor)
                            .await
                    } else {
                        Self::resolve_dynamic_enum_argument(arg, enum_command, current_values, prompt, executor).await
                    }
                } else {
                    Err(ValidationError::Other(t_params!("error_enum_argument_missing_configuration", &[&arg.name]))
                        .into())
                }
            }
            ArgumentType::Text | ArgumentType::Number | ArgumentType::Boolean => {
                Self::resolve_simple_argument(arg, prompt)
            }
        }
    }

    /// Resolve enum argument with static variants
    fn resolve_static_enum_argument(
        arg: &WorkflowArgument,
        variants: &[String],
        prompt: &dyn UserPrompt
    ) -> Result<String, WorkflowError> {
        let prompt_text = t_params!("prompt_select", &[&arg.name]);

        let custom_option = t!("enum_custom_option").to_string();
        let mut options = vec![custom_option.clone()];
        options.extend(variants.iter().cloned());

        let selection = prompt
            .select(&prompt_text, options, PAGE_SIZE)
            .map_err(|e| e.wrap(|msg| ValidationError::SelectionFailed(arg.name.clone(), msg).into()))?;

        if selection == custom_option { Self::prompt_for_custom_value(&arg.name, prompt) } else { Ok(selection) }
    }

    /// Execute a command and parse its output into a list of options.
    /// Handles dynamic_resolution substitution if configured.
    async fn execute_enum_command(
        arg: &WorkflowArgument,
        enum_command: &str,
        current_values: &HashMap<String, String>,
        executor: &dyn CommandExecutor
    ) -> Result<Vec<String>, WorkflowError> {
        let resolved_command = if let Some(ref_arg) = &arg.dynamic_resolution {
            if let Some(ref_value) = current_values.get(ref_arg) {
                enum_command.replace(&format!("{{{{{}}}}}", ref_arg), ref_value)
            } else {
                return Err(ValidationError::Other(t_params!("error_dynamic_resolution_failed", &[ref_arg])).into());
            }
        } else {
            enum_command.to_string()
        };

        let output = executor.execute(&resolved_command).await.map_err(|e| {
            WorkflowError::from(ValidationError::Other(t_params!("error_failed_to_execute_command", &[&e.to_string()])))
        })?;

        let options: Vec<String> = output.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

        if options.is_empty() {
            return Err(ValidationError::Other(t_params!("error_no_options_found", &[&arg.name])).into());
        }

        Ok(options)
    }

    /// Resolve enum argument with dynamic command execution
    async fn resolve_dynamic_enum_argument(
        arg: &WorkflowArgument,
        enum_command: &str,
        current_values: &HashMap<String, String>,
        prompt: &dyn UserPrompt,
        executor: &dyn CommandExecutor
    ) -> Result<String, WorkflowError> {
        let options = Self::execute_enum_command(arg, enum_command, current_values, executor).await?;

        let prompt_text = t_params!("prompt_select", &[&arg.name]);

        let custom_option = t!("enum_custom_option").to_string();
        let mut all_options = vec![custom_option.clone()];
        all_options.extend(options);

        let selection = prompt
            .select(&prompt_text, all_options, PAGE_SIZE)
            .map_err(|e| e.wrap(|msg| ValidationError::SelectionFailed(arg.name.clone(), msg).into()))?;

        if selection == custom_option { Self::prompt_for_custom_value(&arg.name, prompt) } else { Ok(selection) }
    }

    /// Resolve multi-enum argument with static variants
    fn resolve_static_multi_enum_argument(
        arg: &WorkflowArgument,
        variants: &[String],
        prompt: &dyn UserPrompt
    ) -> Result<String, WorkflowError> {
        let prompt_text = t_params!("prompt_multi_select", &[&arg.name]);
        let options: Vec<String> = variants.to_vec();

        let selections = prompt
            .multi_select(&prompt_text, options, PAGE_SIZE, arg.min_selections, arg.max_selections)
            .map_err(|e| e.wrap(|msg| ValidationError::SelectionFailed(arg.name.clone(), msg).into()))?;

        Ok(selections.join(","))
    }

    /// Resolve multi-enum argument with dynamic command execution
    async fn resolve_dynamic_multi_enum_argument(
        arg: &WorkflowArgument,
        enum_command: &str,
        current_values: &HashMap<String, String>,
        prompt: &dyn UserPrompt,
        executor: &dyn CommandExecutor
    ) -> Result<String, WorkflowError> {
        let options = Self::execute_enum_command(arg, enum_command, current_values, executor).await?;

        let prompt_text = t_params!("prompt_multi_select", &[&arg.name]);

        let selections = prompt
            .multi_select(&prompt_text, options, PAGE_SIZE, arg.min_selections, arg.max_selections)
            .map_err(|e| e.wrap(|msg| ValidationError::SelectionFailed(arg.name.clone(), msg).into()))?;

        Ok(selections.join(","))
    }

    /// Resolve simple text/number/boolean argument
    fn resolve_simple_argument(arg: &WorkflowArgument, prompt: &dyn UserPrompt) -> Result<String, WorkflowError> {
        let prompt_text = t_params!("prompt_enter", &[&arg.name]);

        let default = arg.default_value.as_deref().filter(|d| !d.is_empty() && *d != "~");

        prompt
            .text(&prompt_text, default)
            .map_err(|e| e.wrap(|msg| ValidationError::InputFailed(arg.name.clone(), msg).into()))
    }

    /// Prompt user for a custom value
    fn prompt_for_custom_value(arg_name: &str, prompt: &dyn UserPrompt) -> Result<String, WorkflowError> {
        let custom_prompt = t_params!("enum_enter_custom_value", &[arg_name]);
        prompt
            .text(&custom_prompt, None)
            .map_err(|e| e.wrap(|msg| ValidationError::InputFailed(arg_name.to_string(), msg).into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        adapter::{
            executor::mock::MockExecutor,
            prompt::mock::{MockPrompt, MockPromptResponse}
        },
        domain::workflow::{ArgumentType, WorkflowArgument}
    };

    fn text_arg(name: &str) -> WorkflowArgument {
        WorkflowArgument {
            name:               name.to_string(),
            description:        format!("{} description", name),
            arg_type:           ArgumentType::Text,
            default_value:      None,
            enum_variants:      None,
            enum_command:       None,
            enum_name:          None,
            dynamic_resolution: None,
            multi:              false,
            min_selections:     None,
            max_selections:     None
        }
    }

    fn enum_arg(name: &str, variants: Vec<String>) -> WorkflowArgument {
        WorkflowArgument {
            name:               name.to_string(),
            description:        format!("{} description", name),
            arg_type:           ArgumentType::Enum,
            default_value:      None,
            enum_variants:      Some(variants),
            enum_command:       None,
            enum_name:          None,
            dynamic_resolution: None,
            multi:              false,
            min_selections:     None,
            max_selections:     None
        }
    }

    fn multi_enum_arg(name: &str, variants: Vec<String>) -> WorkflowArgument {
        WorkflowArgument {
            name:               name.to_string(),
            description:        format!("{} description", name),
            arg_type:           ArgumentType::Enum,
            default_value:      None,
            enum_variants:      Some(variants),
            enum_command:       None,
            enum_name:          None,
            dynamic_resolution: None,
            multi:              true,
            min_selections:     None,
            max_selections:     None
        }
    }

    fn dynamic_enum_arg(name: &str, command: &str, enum_name: &str) -> WorkflowArgument {
        WorkflowArgument {
            name:               name.to_string(),
            description:        format!("{} description", name),
            arg_type:           ArgumentType::Enum,
            default_value:      None,
            enum_variants:      None,
            enum_command:       Some(command.to_string()),
            enum_name:          Some(enum_name.to_string()),
            dynamic_resolution: None,
            multi:              false,
            min_selections:     None,
            max_selections:     None
        }
    }

    #[tokio::test]
    async fn resolve_text_argument() {
        let prompt = MockPrompt::new(vec![MockPromptResponse::Text("my-value".to_string())]);
        let executor = MockExecutor::new(HashMap::new());
        let args = vec![text_arg("project_name")];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("project_name").unwrap(), "my-value");
    }

    #[tokio::test]
    async fn resolve_static_enum_argument() {
        let prompt = MockPrompt::new(vec![MockPromptResponse::Select("prod".to_string())]);
        let executor = MockExecutor::new(HashMap::new());
        let args = vec![enum_arg("env", vec!["dev".into(), "staging".into(), "prod".into()])];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("env").unwrap(), "prod");
    }

    #[tokio::test]
    async fn resolve_static_multi_enum_argument() {
        let prompt =
            MockPrompt::new(vec![MockPromptResponse::MultiSelect(vec!["feat-a".to_string(), "feat-c".to_string()])]);
        let executor = MockExecutor::new(HashMap::new());
        let args = vec![multi_enum_arg("features", vec!["feat-a".into(), "feat-b".into(), "feat-c".into()])];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("features").unwrap(), "feat-a,feat-c");
    }

    #[tokio::test]
    async fn resolve_dynamic_enum_argument() {
        let prompt = MockPrompt::new(vec![MockPromptResponse::Select("branch-2".to_string())]);
        let mut cmd_responses = HashMap::new();
        cmd_responses.insert("list-branches".to_string(), Ok("branch-1\nbranch-2\nbranch-3\n".to_string()));
        let executor = MockExecutor::new(cmd_responses);
        let args = vec![dynamic_enum_arg("branch", "list-branches", "branches")];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("branch").unwrap(), "branch-2");
    }

    #[tokio::test]
    async fn resolve_multiple_arguments_in_order() {
        let prompt = MockPrompt::new(vec![
            MockPromptResponse::Text("my-project".to_string()),
            MockPromptResponse::Select("prod".to_string()),
        ]);
        let executor = MockExecutor::new(HashMap::new());
        let args = vec![text_arg("name"), enum_arg("env", vec!["dev".into(), "prod".into()])];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("name").unwrap(), "my-project");
        assert_eq!(result.get("env").unwrap(), "prod");
    }

    #[tokio::test]
    async fn resolve_enum_missing_config_errors() {
        let prompt = MockPrompt::new(vec![]);
        let executor = MockExecutor::new(HashMap::new());
        let args = vec![WorkflowArgument {
            name:               "bad".to_string(),
            description:        "bad".to_string(),
            arg_type:           ArgumentType::Enum,
            default_value:      None,
            enum_variants:      None,
            enum_command:       None,
            enum_name:          None,
            dynamic_resolution: None,
            multi:              false,
            min_selections:     None,
            max_selections:     None
        }];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn resolve_empty_arguments_returns_empty_map() {
        let prompt = MockPrompt::new(vec![]);
        let executor = MockExecutor::new(HashMap::new());

        let result = ArgumentResolver::resolve_workflow_arguments(&[], &prompt, &executor).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn enum_without_multi_uses_single_select() {
        let prompt = MockPrompt::new(vec![MockPromptResponse::Select("prod".to_string())]);
        let executor = MockExecutor::new(HashMap::new());
        let args = vec![enum_arg("env", vec!["dev".into(), "prod".into()])];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("env").unwrap(), "prod");
    }

    #[tokio::test]
    async fn enum_with_multi_uses_multi_select() {
        let prompt = MockPrompt::new(vec![MockPromptResponse::MultiSelect(vec!["api".to_string(), "web".to_string()])]);
        let executor = MockExecutor::new(HashMap::new());
        let args = vec![multi_enum_arg("services", vec!["api".into(), "web".into(), "worker".into()])];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("services").unwrap(), "api,web");
    }

    #[tokio::test]
    async fn enum_multi_without_constraints_works() {
        let prompt = MockPrompt::new(vec![MockPromptResponse::MultiSelect(vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
        ])]);
        let executor = MockExecutor::new(HashMap::new());
        let args = vec![WorkflowArgument {
            name:               "items".to_string(),
            description:        "pick items".to_string(),
            arg_type:           ArgumentType::Enum,
            default_value:      None,
            enum_variants:      Some(vec!["a".into(), "b".into(), "c".into()]),
            enum_command:       None,
            enum_name:          None,
            dynamic_resolution: None,
            multi:              true,
            min_selections:     None,
            max_selections:     None
        }];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("items").unwrap(), "a,b,c");
    }

    #[tokio::test]
    async fn enum_multi_dynamic_uses_multi_select() {
        let prompt =
            MockPrompt::new(vec![MockPromptResponse::MultiSelect(vec!["ns-a".to_string(), "ns-b".to_string()])]);
        let mut cmd_responses = HashMap::new();
        cmd_responses.insert("list-ns".to_string(), Ok("ns-a\nns-b\nns-c\n".to_string()));
        let executor = MockExecutor::new(cmd_responses);
        let args = vec![WorkflowArgument {
            name:               "namespaces".to_string(),
            description:        "pick namespaces".to_string(),
            arg_type:           ArgumentType::Enum,
            default_value:      None,
            enum_variants:      None,
            enum_command:       Some("list-ns".to_string()),
            enum_name:          Some("ns".to_string()),
            dynamic_resolution: None,
            multi:              true,
            min_selections:     None,
            max_selections:     None
        }];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("namespaces").unwrap(), "ns-a,ns-b");
    }
}
