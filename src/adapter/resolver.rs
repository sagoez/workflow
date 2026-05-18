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
    port::{
        executor::CommandExecutor,
        prompt::{SelectOption, UserPrompt}
    },
    t, t_params
};

/// Parse a default boolean value from the workflow YAML's `default_value: "..."` string.
/// Recognized true: "true", "yes", "y", "1". Recognized false: "false", "no", "n", "0".
/// Case-insensitive. Returns None for "~", empty, or unrecognized input.
fn parse_bool_default(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "y" | "1" => Some(true),
        "false" | "no" | "n" | "0" => Some(false),
        _ => None
    }
}

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
                } else if let Some(enum_command) = &arg.enum_command {
                    if multi {
                        Self::resolve_dynamic_multi_enum_argument(arg, enum_command, current_values, prompt, executor)
                            .await
                    } else {
                        Self::resolve_dynamic_enum_argument(arg, enum_command, current_values, prompt, executor).await
                    }
                } else {
                    Err(ValidationError::EnumMissingConfig(arg.name.clone()).into())
                }
            }
            ArgumentType::Text => Self::resolve_simple_argument(arg, prompt),
            ArgumentType::Number => Self::resolve_number_argument(arg, prompt),
            ArgumentType::Boolean => Self::resolve_boolean_argument(arg, prompt)
        }
    }

    /// Resolve a numeric argument. Returns an InputFailed error with a translated
    /// "not a valid number" message if the input doesn't parse as an f64.
    fn resolve_number_argument(arg: &WorkflowArgument, prompt: &dyn UserPrompt) -> Result<String, WorkflowError> {
        let prompt_text = t_params!("prompt_enter_number", &[&arg.name]);
        let default = arg.default_value.as_deref().filter(|d| !d.is_empty() && *d != "~");

        let raw = prompt.text(&prompt_text, default)?;

        if raw.parse::<f64>().is_ok() {
            Ok(raw)
        } else {
            Err(ValidationError::InputFailed(arg.name.clone(), t_params!("error_invalid_number", &[&raw])).into())
        }
    }

    /// Resolve a boolean argument via a yes/no confirm prompt.
    fn resolve_boolean_argument(arg: &WorkflowArgument, prompt: &dyn UserPrompt) -> Result<String, WorkflowError> {
        let prompt_text = t_params!("prompt_confirm_boolean", &[&arg.name]);
        let default = arg.default_value.as_deref().and_then(parse_bool_default).unwrap_or(false);

        let value = prompt.confirm(&prompt_text, default)?;

        Ok(if value { "true".to_string() } else { "false".to_string() })
    }

    /// Resolve enum argument with static variants
    fn resolve_static_enum_argument(
        arg: &WorkflowArgument,
        variants: &[String],
        prompt: &dyn UserPrompt
    ) -> Result<String, WorkflowError> {
        let prompt_text = t_params!("prompt_select", &[&arg.name]);

        let custom_option = t!("enum_custom_option").to_string();
        let mut options: Vec<SelectOption> = vec![SelectOption::plain(custom_option.clone())];
        options.extend(variants.iter().cloned().map(SelectOption::plain));

        let selection = prompt
            .select(&prompt_text, options, PAGE_SIZE)
            .map_err(|e| WorkflowError::from(ValidationError::SelectionFailed(arg.name.clone(), e.to_string())))?;

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
                return Err(ValidationError::DynamicResolutionFailed(ref_arg.clone()).into());
            }
        } else {
            enum_command.to_string()
        };

        let output = executor
            .execute(&resolved_command)
            .await
            .map_err(|e| WorkflowError::Execution(t_params!("error_failed_to_execute_command", &[&e.to_string()])))?;

        let options: Vec<String> = output.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

        if options.is_empty() {
            return Err(ValidationError::NoOptionsFound(arg.name.clone()).into());
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
        let mut all_options: Vec<SelectOption> = vec![SelectOption::plain(custom_option.clone())];
        all_options.extend(options.into_iter().map(SelectOption::plain));

        let selection = prompt
            .select(&prompt_text, all_options, PAGE_SIZE)
            .map_err(|e| WorkflowError::from(ValidationError::SelectionFailed(arg.name.clone(), e.to_string())))?;

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
            .map_err(|e| WorkflowError::from(ValidationError::SelectionFailed(arg.name.clone(), e.to_string())))?;

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
            .map_err(|e| WorkflowError::from(ValidationError::SelectionFailed(arg.name.clone(), e.to_string())))?;

        Ok(selections.join(","))
    }

    /// Resolve simple text/number/boolean argument
    fn resolve_simple_argument(arg: &WorkflowArgument, prompt: &dyn UserPrompt) -> Result<String, WorkflowError> {
        let prompt_text = t_params!("prompt_enter", &[&arg.name]);

        let default = arg.default_value.as_deref().filter(|d| !d.is_empty() && *d != "~");

        prompt
            .text(&prompt_text, default)
            .map_err(|e| WorkflowError::from(ValidationError::InputFailed(arg.name.clone(), e.to_string())))
    }

    /// Prompt user for a custom value
    fn prompt_for_custom_value(arg_name: &str, prompt: &dyn UserPrompt) -> Result<String, WorkflowError> {
        let custom_prompt = t_params!("enum_enter_custom_value", &[arg_name]);
        prompt
            .text(&custom_prompt, None)
            .map_err(|e| ValidationError::InputFailed(arg_name.to_string(), e.to_string()).into())
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

    fn number_arg(name: &str, default: Option<&str>) -> WorkflowArgument {
        WorkflowArgument {
            name:               name.to_string(),
            description:        format!("{} description", name),
            arg_type:           ArgumentType::Number,
            default_value:      default.map(String::from),
            enum_variants:      None,
            enum_command:       None,
            enum_name:          None,
            dynamic_resolution: None,
            multi:              false,
            min_selections:     None,
            max_selections:     None
        }
    }

    fn boolean_arg(name: &str, default: Option<&str>) -> WorkflowArgument {
        WorkflowArgument {
            name:               name.to_string(),
            description:        format!("{} description", name),
            arg_type:           ArgumentType::Boolean,
            default_value:      default.map(String::from),
            enum_variants:      None,
            enum_command:       None,
            enum_name:          None,
            dynamic_resolution: None,
            multi:              false,
            min_selections:     None,
            max_selections:     None
        }
    }

    #[tokio::test]
    async fn resolve_number_argument_accepts_integer() {
        let prompt = MockPrompt::new(vec![MockPromptResponse::Text("42".to_string())]);
        let executor = MockExecutor::new(HashMap::new());
        let args = vec![number_arg("port", None)];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("port").unwrap(), "42");
    }

    #[tokio::test]
    async fn resolve_number_argument_accepts_float() {
        let prompt = MockPrompt::new(vec![MockPromptResponse::Text("3.14".to_string())]);
        let executor = MockExecutor::new(HashMap::new());
        let args = vec![number_arg("ratio", None)];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("ratio").unwrap(), "3.14");
    }

    #[tokio::test]
    async fn resolve_number_argument_rejects_non_numeric() {
        let prompt = MockPrompt::new(vec![MockPromptResponse::Text("abc".to_string())]);
        let executor = MockExecutor::new(HashMap::new());
        let args = vec![number_arg("port", None)];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn resolve_boolean_argument_true() {
        let prompt = MockPrompt::new(vec![MockPromptResponse::Confirm(true)]);
        let executor = MockExecutor::new(HashMap::new());
        let args = vec![boolean_arg("enabled", None)];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("enabled").unwrap(), "true");
    }

    #[tokio::test]
    async fn resolve_boolean_argument_false() {
        let prompt = MockPrompt::new(vec![MockPromptResponse::Confirm(false)]);
        let executor = MockExecutor::new(HashMap::new());
        let args = vec![boolean_arg("enabled", Some("true"))];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("enabled").unwrap(), "false");
    }

    #[test]
    fn parse_bool_default_recognizes_truthy() {
        for input in ["true", "True", "TRUE", "yes", "y", "1", " true "] {
            assert_eq!(parse_bool_default(input), Some(true), "input: {:?}", input);
        }
    }

    #[test]
    fn parse_bool_default_recognizes_falsy() {
        for input in ["false", "False", "no", "n", "0"] {
            assert_eq!(parse_bool_default(input), Some(false), "input: {:?}", input);
        }
    }

    #[test]
    fn parse_bool_default_returns_none_for_unknown() {
        for input in ["", "~", "maybe", "tru", "12"] {
            assert_eq!(parse_bool_default(input), None, "input: {:?}", input);
        }
    }

    #[tokio::test]
    async fn enum_with_command_but_no_enum_name_resolves() {
        let prompt = MockPrompt::new(vec![MockPromptResponse::Select("ns-a".to_string())]);
        let mut cmd_responses = HashMap::new();
        cmd_responses.insert("list-ns".to_string(), Ok("ns-a\nns-b\n".to_string()));
        let executor = MockExecutor::new(cmd_responses);
        let args = vec![WorkflowArgument {
            name:               "namespace".to_string(),
            description:        "pick".to_string(),
            arg_type:           ArgumentType::Enum,
            default_value:      None,
            enum_variants:      None,
            enum_command:       Some("list-ns".to_string()),
            enum_name:          None, // not provided — should still work
            dynamic_resolution: None,
            multi:              false,
            min_selections:     None,
            max_selections:     None
        }];

        let result = ArgumentResolver::resolve_workflow_arguments(&args, &prompt, &executor).await.unwrap();
        assert_eq!(result.get("namespace").unwrap(), "ns-a");
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
