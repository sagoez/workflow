use async_trait::async_trait;
use tokio::process::Command as TokioCommand;

use crate::{domain::error::WorkflowError, port::executor::CommandExecutor, t_params};

/// Real implementation wrapping TokioCommand
pub struct ShellExecutor;

impl ShellExecutor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CommandExecutor for ShellExecutor {
    async fn execute(&self, command: &str) -> Result<String, WorkflowError> {
        let output =
            TokioCommand::new("sh").arg("-c").arg(command).output().await.map_err(|e| {
                WorkflowError::Execution(t_params!("error_failed_to_execute_command", &[&e.to_string()]))
            })?;

        if !output.status.success() {
            return Err(WorkflowError::Execution(t_params!(
                "error_command_failed",
                &[&String::from_utf8_lossy(&output.stderr)]
            )));
        }

        String::from_utf8(output.stdout)
            .map_err(|e| WorkflowError::Execution(t_params!("error_failed_to_parse_command_output", &[&e.to_string()])))
    }
}

#[cfg(test)]
pub mod mock {
    use std::{collections::HashMap, sync::Mutex};

    use super::*;

    /// Mock executor that returns pre-configured outputs for specific commands
    pub struct MockExecutor {
        responses: Mutex<HashMap<String, Result<String, WorkflowError>>>
    }

    impl MockExecutor {
        pub fn new(responses: HashMap<String, Result<String, WorkflowError>>) -> Self {
            Self { responses: Mutex::new(responses) }
        }
    }

    #[async_trait]
    impl CommandExecutor for MockExecutor {
        async fn execute(&self, command: &str) -> Result<String, WorkflowError> {
            let responses = self.responses.lock().unwrap();
            match responses.get(command) {
                Some(Ok(output)) => Ok(output.clone()),
                Some(Err(e)) => Err(e.clone()),
                None => Err(WorkflowError::Execution(format!("MockExecutor: no response configured for '{}'", command)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{mock::*, *};

    #[tokio::test]
    async fn mock_executor_returns_configured_output() {
        let mut responses = HashMap::new();
        responses.insert("echo hello".to_string(), Ok("hello\n".to_string()));
        let mock = MockExecutor::new(responses);
        let result = mock.execute("echo hello").await.unwrap();
        assert_eq!(result, "hello\n");
    }

    #[tokio::test]
    async fn mock_executor_returns_configured_error() {
        let mut responses = HashMap::new();
        responses.insert("fail-cmd".to_string(), Err(WorkflowError::Execution("command failed".to_string())));
        let mock = MockExecutor::new(responses);
        let result = mock.execute("fail-cmd").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mock_executor_unknown_command_errors() {
        let mock = MockExecutor::new(HashMap::new());
        let result = mock.execute("unknown").await;
        assert!(result.is_err());
    }
}
