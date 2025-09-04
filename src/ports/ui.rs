//! UI ports - interfaces for user interaction

use std::collections::HashMap;

use async_trait::async_trait;

use crate::shared::WorkflowError;

/// Port for prompting user input
#[async_trait]
pub trait Prompter: Send + Sync {
    /// Prompt for text input with optional default
    async fn prompt_text(&self, message: &str, default: Option<&str>) -> Result<String, WorkflowError>;

    /// Prompt for selection from a list of options
    async fn prompt_select(&self, message: &str, options: Vec<String>) -> Result<String, WorkflowError>;

    /// Prompt for confirmation (yes/no)
    async fn prompt_confirm(&self, message: &str, default: Option<bool>) -> Result<bool, WorkflowError>;

    /// Prompt for password input (hidden)
    async fn prompt_password(&self, message: &str) -> Result<String, WorkflowError>;

    /// Prompt for multiple arguments based on workflow definition
    async fn prompt_arguments(
        &self,
        arguments: &[ArgumentDefinition]
    ) -> Result<HashMap<String, String>, WorkflowError>;
}

/// Port for rendering output to the user
#[async_trait]
pub trait Renderer: Send + Sync {
    /// Display a message to the user
    async fn display_message(&self, message: &str) -> Result<(), WorkflowError>;

    /// Display an error message
    async fn display_error(&self, error: &str) -> Result<(), WorkflowError>;

    /// Display a success message
    async fn display_success(&self, message: &str) -> Result<(), WorkflowError>;

    /// Display a warning message
    async fn display_warning(&self, message: &str) -> Result<(), WorkflowError>;

    /// Display workflow information
    async fn display_workflow_info(&self, workflow: &WorkflowInfo) -> Result<(), WorkflowError>;

    /// Display a list of workflows
    async fn display_workflow_list(&self, workflows: &[WorkflowInfo]) -> Result<(), WorkflowError>;

    /// Display command output
    async fn display_command(&self, command: &str) -> Result<(), WorkflowError>;
}

/// Port for showing progress indicators
#[async_trait]
pub trait ProgressIndicator: Send + Sync {
    /// Start a spinner with a message
    async fn start_spinner(&self, message: &str) -> Result<Box<dyn SpinnerHandle>, WorkflowError>;

    /// Start a progress bar with total steps
    async fn start_progress(&self, message: &str, total: u64) -> Result<Box<dyn ProgressHandle>, WorkflowError>;

    /// Update progress
    async fn update_progress(
        &self,
        handle: &dyn ProgressHandle,
        current: u64,
        message: Option<&str>
    ) -> Result<(), WorkflowError>;

    /// Finish progress indicator
    async fn finish_progress(&self, handle: &dyn ProgressHandle, message: Option<&str>) -> Result<(), WorkflowError>;
}

/// Handle for managing spinner state
pub trait SpinnerHandle: Send + Sync {
    fn update_message(&self, message: &str);
    fn finish(&self, message: Option<&str>);
    fn finish_and_clear(&self);
}

/// Handle for managing progress bar state
pub trait ProgressHandle: Send + Sync {
    fn set_position(&self, position: u64);
    fn set_message(&self, message: &str);
    fn finish(&self, message: Option<&str>);
    fn finish_and_clear(&self);
}

/// Argument definition for prompting
#[derive(Debug, Clone)]
pub struct ArgumentDefinition {
    pub name:          String,
    pub description:   String,
    pub arg_type:      ArgumentType,
    pub default_value: Option<String>,
    pub options:       Option<Vec<String>> // For enum types
}

/// Types of arguments that can be prompted
#[derive(Debug, Clone)]
pub enum ArgumentType {
    Text,
    Number,
    Boolean,
    Enum,
    Password
}

/// Workflow information for display
#[derive(Debug, Clone)]
pub struct WorkflowInfo {
    pub name:           String,
    pub file_path:      String,
    pub description:    String,
    pub tags:           Vec<String>,
    pub argument_count: usize
}
