//! User input prompts and interactions

use anyhow::{Context, Result};
use inquire::{Select, Text};
use crate::{i18n, utils::FilePathCompleter};

/// Show a selection menu for enum options
pub fn show_enum_selection(prompt: &str, options: Vec<String>) -> Result<String> {
    let selection = Select::new(prompt, options)
        .with_page_size(10)
        .with_help_message(&i18n::t("enum_args_searchable_help"))
        .prompt()
        .with_context(|| i18n::t("errors_selection_failed"))?;

    Ok(selection)
}

/// Show a text input prompt with optional default
pub fn show_text_input(prompt: &str, default: Option<&str>) -> Result<String> {
    let mut text_input = Text::new(prompt).with_autocomplete(FilePathCompleter::default());

    if let Some(default_value) = default {
        if !default_value.is_empty() && default_value != "~" {
            text_input = text_input.with_default(default_value);
        }
    }

    text_input.prompt().with_context(|| i18n::t_params("simple_args_input_failed", &[prompt]))
}

/// Show an interactive workflow selection menu
pub fn show_workflow_selection_menu(workflow_names: Vec<String>) -> Result<usize> {
    let selection = Select::new(&i18n::t("cli_select_prompt"), workflow_names.clone())
        .with_page_size(10)
        .prompt()
        .with_context(|| "Failed to show workflow selection")?;

    let selection_index = workflow_names
        .iter()
        .position(|name| name == &selection)
        .unwrap_or(0);

    Ok(selection_index)
}
