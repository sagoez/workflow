//! Argument resolution functionality

use std::{collections::HashMap, process::Command};
use anyhow::{Context, Result};

use crate::{i18n, ui};
use super::{WorkflowArgument, ArgumentType};

/// Resolve all workflow arguments by prompting the user
pub async fn resolve_workflow_arguments(arguments: &[WorkflowArgument]) -> Result<HashMap<String, String>> {
    let mut argument_values = HashMap::new();

    for arg in arguments {
        let value = resolve_argument(arg, &argument_values).await?;
        argument_values.insert(arg.name.clone(), value);
    }

    Ok(argument_values)
}

/// Resolve a single argument based on its type.
///
/// Dispatches to the appropriate resolution method:
/// - Enum arguments: Execute command and show selection menu
/// - Other types: Show input prompt with optional default
///
/// # Arguments
/// * `arg` - The argument definition to resolve
/// * `current_values` - Currently resolved argument values for dynamic resolution
///
/// # Returns
/// * `Ok(String)` - The resolved argument value
/// * `Err(anyhow::Error)` - Error during resolution
async fn resolve_argument(
    arg: &WorkflowArgument,
    current_values: &HashMap<String, String>
) -> Result<String> {
    match arg.arg_type {
        ArgumentType::Enum => {
            if let Some(enum_variants) = &arg.enum_variants {
                // Static enum variants
                resolve_static_enum_argument(arg, enum_variants)
            } else if let (Some(enum_command), Some(enum_name)) = (&arg.enum_command, &arg.enum_name) {
                // Dynamic enum via command
                resolve_enum_argument(arg, enum_command, enum_name, current_values).await
            } else {
                anyhow::bail!(i18n::t_params("enum_args_missing_config", &[&arg.name]));
            }
        }
        ArgumentType::Text | ArgumentType::Number | ArgumentType::Boolean => resolve_simple_argument(arg)
    }
}

/// Resolve an enum argument using static predefined variants.
///
/// This method presents a searchable selection menu with the predefined options
/// from the `enum_variants` field, allowing users to search, select, or type custom values.
///
/// # Arguments
/// * `arg` - The argument definition
/// * `variants` - The predefined list of options
///
/// # Returns
/// * `Ok(String)` - The selected option or custom input
/// * `Err(anyhow::Error)` - User interaction error
fn resolve_static_enum_argument(arg: &WorkflowArgument, variants: &[String]) -> Result<String> {
    if variants.is_empty() {
        anyhow::bail!(i18n::t_params("enum_args_no_options_found", &[&arg.name]));
    }

    let prompt = if arg.description.is_empty() || arg.description == "~" {
        arg.name.clone()
    } else {
        arg.description.clone()
    };

    ui::show_enum_selection(&prompt, variants.to_vec())
}

/// Resolve an enum argument by executing a command and presenting options.
///
/// This method:
/// 1. Shows a spinner while executing the enum command
/// 2. Parses the command output into selectable options
/// 3. Presents a searchable selection menu allowing search, selection, or custom input
/// 4. Supports dynamic resolution using previously resolved arguments
///
/// # Arguments
/// * `arg` - The argument definition containing description
/// * `enum_command` - Shell command to execute to get options
/// * `_enum_name` - Identifier for the enum (unused but kept for future features)
/// * `current_values` - Currently resolved argument values for dynamic resolution
///
/// # Returns
/// * `Ok(String)` - The selected option value or custom input
/// * `Err(anyhow::Error)` - Command execution failure or user interaction error
async fn resolve_enum_argument(
    arg: &WorkflowArgument,
    enum_command: &str,
    _enum_name: &str,
    current_values: &HashMap<String, String>
) -> Result<String> {
    let spinner = ui::create_enum_spinner(&i18n::t_params("enum_args_getting_options", &[&arg.name]));

    // Handle dynamic resolution if specified
    let resolved_command = if let Some(ref_arg) = &arg.dynamic_resolution {
        if let Some(ref_value) = current_values.get(ref_arg) {
            // Substitute the referenced argument value in the enum_command
            enum_command.replace(&format!("{{{{{}}}}}", ref_arg), ref_value)
        } else {
            anyhow::bail!("Dynamic resolution failed: referenced argument '{}' not found", ref_arg);
        }
    } else {
        enum_command.to_string()
    };

    let output = Command::new("sh")
        .arg("-c")
        .arg(&resolved_command)
        .output()
        .with_context(|| i18n::t_params("errors_enum_command_execution_failed", &[&resolved_command]))?;

    spinner.finish_and_clear();

    if !output.status.success() {
        anyhow::bail!(i18n::t_params("enum_args_command_failed", &[&String::from_utf8_lossy(&output.stderr)]));
    }

    let options: Vec<String> =
        String::from_utf8(output.stdout)?.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

    if options.is_empty() {
        anyhow::bail!(i18n::t_params("enum_args_no_options_found", &[&arg.name]));
    }

    let prompt = if arg.description.is_empty() || arg.description == "~" {
        arg.name.clone()
    } else {
        arg.description.clone()
    };

    ui::show_enum_selection(&prompt, options)
}

/// Resolve a simple argument (Text, Number, Boolean) through user input.
///
/// Presents an interactive input prompt with:
/// - Argument name and description
/// - Default value if available and not null (~)
/// - Input validation for the argument type
///
/// # Arguments
/// * `arg` - The argument definition to resolve
///
/// # Returns
/// * `Ok(String)` - The user-provided or default value
/// * `Err(anyhow::Error)` - User interaction error
fn resolve_simple_argument(arg: &WorkflowArgument) -> Result<String> {
    let prompt = if arg.description.is_empty() || arg.description == "~" {
        arg.name.clone()
    } else {
        arg.description.clone()
    };

    let default = arg.default_value.as_deref();
    ui::show_text_input(&prompt, default)
}
