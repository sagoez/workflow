use async_trait::async_trait;

use crate::domain::error::WorkflowError;

/// Port trait for shell command execution
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    /// Execute a shell command and return its stdout on success
    async fn execute(&self, command: &str) -> Result<String, WorkflowError>;
}
