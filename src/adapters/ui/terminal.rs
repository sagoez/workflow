//! Terminal-based implementation of UI ports

use std::collections::HashMap;

use async_trait::async_trait;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{Confirm, Password, Select, Text};

use crate::{
    ports::ui::{
        ArgumentDefinition, ArgumentType, ProgressHandle, ProgressIndicator, Prompter, Renderer, SpinnerHandle,
        WorkflowInfo
    },
    shared::WorkflowError
};

/// Terminal implementation of Prompter
pub struct TerminalPrompter;

impl TerminalPrompter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Prompter for TerminalPrompter {
    async fn prompt_text(&self, message: &str, default: Option<&str>) -> Result<String, WorkflowError> {
        let mut prompt = Text::new(message);

        if let Some(default_value) = default {
            prompt = prompt.with_default(default_value);
        }

        prompt.prompt().map_err(|e| WorkflowError::UserInteraction(e.to_string()))
    }

    async fn prompt_select(&self, message: &str, options: Vec<String>) -> Result<String, WorkflowError> {
        Select::new(message, options)
            .with_page_size(10)
            .prompt()
            .map_err(|e| WorkflowError::UserInteraction(e.to_string()))
    }

    async fn prompt_confirm(&self, message: &str, default: Option<bool>) -> Result<bool, WorkflowError> {
        let mut prompt = Confirm::new(message);

        if let Some(default_value) = default {
            prompt = prompt.with_default(default_value);
        }

        prompt.prompt().map_err(|e| WorkflowError::UserInteraction(e.to_string()))
    }

    async fn prompt_password(&self, message: &str) -> Result<String, WorkflowError> {
        Password::new(message)
            .without_confirmation()
            .prompt()
            .map_err(|e| WorkflowError::UserInteraction(e.to_string()))
    }

    async fn prompt_arguments(
        &self,
        arguments: &[ArgumentDefinition]
    ) -> Result<HashMap<String, String>, WorkflowError> {
        let mut results = HashMap::new();

        for arg in arguments {
            let value = match arg.arg_type {
                ArgumentType::Text | ArgumentType::Number => {
                    self.prompt_text(&arg.description, arg.default_value.as_deref()).await?
                }
                ArgumentType::Boolean => {
                    let default = arg.default_value.as_ref().and_then(|v| v.parse::<bool>().ok());
                    let result = self.prompt_confirm(&arg.description, default).await?;
                    result.to_string()
                }
                ArgumentType::Enum => {
                    if let Some(options) = &arg.options {
                        self.prompt_select(&arg.description, options.clone()).await?
                    } else {
                        return Err(WorkflowError::Validation("Enum argument missing options".to_string()));
                    }
                }
                ArgumentType::Password => self.prompt_password(&arg.description).await?
            };

            results.insert(arg.name.clone(), value);
        }

        Ok(results)
    }
}

/// Terminal implementation of Renderer
pub struct TerminalRenderer;

impl TerminalRenderer {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Renderer for TerminalRenderer {
    async fn display_message(&self, message: &str) -> Result<(), WorkflowError> {
        println!("{}", message);
        Ok(())
    }

    async fn display_error(&self, error: &str) -> Result<(), WorkflowError> {
        eprintln!("❌ {}", error);
        Ok(())
    }

    async fn display_success(&self, message: &str) -> Result<(), WorkflowError> {
        println!("✅ {}", message);
        Ok(())
    }

    async fn display_warning(&self, message: &str) -> Result<(), WorkflowError> {
        println!("⚠️  {}", message);
        Ok(())
    }

    async fn display_workflow_info(&self, workflow: &WorkflowInfo) -> Result<(), WorkflowError> {
        println!("🔧 {}", workflow.name);
        println!("   Description: {}", workflow.description);
        println!("   File: {}", workflow.file_path);
        println!("   Arguments: {}", workflow.argument_count);
        if !workflow.tags.is_empty() {
            println!("   Tags: {}", workflow.tags.join(", "));
        }
        Ok(())
    }

    async fn display_workflow_list(&self, workflows: &[WorkflowInfo]) -> Result<(), WorkflowError> {
        println!("📋 Available workflows:\n");

        for workflow in workflows {
            self.display_workflow_info(workflow).await?;
            println!();
        }

        Ok(())
    }

    async fn display_command(&self, command: &str) -> Result<(), WorkflowError> {
        println!();
        println!("$ {}", command);
        println!();
        Ok(())
    }
}

/// Terminal implementation of ProgressIndicator
pub struct TerminalProgressIndicator;

impl TerminalProgressIndicator {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ProgressIndicator for TerminalProgressIndicator {
    async fn start_spinner(&self, message: &str) -> Result<Box<dyn SpinnerHandle>, WorkflowError> {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .expect("Failed to create spinner style")
        );
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));

        Ok(Box::new(TerminalSpinnerHandle { spinner }))
    }

    async fn start_progress(&self, message: &str, total: u64) -> Result<Box<dyn ProgressHandle>, WorkflowError> {
        let progress = ProgressBar::new(total);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("Failed to create progress style")
        );
        progress.set_message(message.to_string());

        Ok(Box::new(TerminalProgressHandle { progress }))
    }

    async fn update_progress(
        &self,
        handle: &dyn ProgressHandle,
        current: u64,
        message: Option<&str>
    ) -> Result<(), WorkflowError> {
        handle.set_position(current);
        if let Some(msg) = message {
            handle.set_message(msg);
        }
        Ok(())
    }

    async fn finish_progress(&self, handle: &dyn ProgressHandle, message: Option<&str>) -> Result<(), WorkflowError> {
        handle.finish(message);
        Ok(())
    }
}

/// Terminal spinner handle
struct TerminalSpinnerHandle {
    spinner: ProgressBar
}

impl SpinnerHandle for TerminalSpinnerHandle {
    fn update_message(&self, message: &str) {
        self.spinner.set_message(message.to_string());
    }

    fn finish(&self, message: Option<&str>) {
        if let Some(msg) = message {
            self.spinner.finish_with_message(msg.to_string());
        } else {
            self.spinner.finish();
        }
    }

    fn finish_and_clear(&self) {
        self.spinner.finish_and_clear();
    }
}

/// Terminal progress handle
struct TerminalProgressHandle {
    progress: ProgressBar
}

impl ProgressHandle for TerminalProgressHandle {
    fn set_position(&self, position: u64) {
        self.progress.set_position(position);
    }

    fn set_message(&self, message: &str) {
        self.progress.set_message(message.to_string());
    }

    fn finish(&self, message: Option<&str>) {
        if let Some(msg) = message {
            self.progress.finish_with_message(msg.to_string());
        } else {
            self.progress.finish();
        }
    }

    fn finish_and_clear(&self) {
        self.progress.finish_and_clear();
    }
}
