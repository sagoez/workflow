//! Common types and value objects used throughout the system

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Command execution context providing dependencies and environment
#[derive(Clone, Debug)]
pub struct CommandContext {
    /// Unique session identifier
    pub session_id: String,

    /// Current user (from system)
    pub user: String,

    /// Current hostname
    pub hostname: String,

    /// Current working directory
    pub working_directory: String,

    /// Command execution timestamp
    pub timestamp: DateTime<Utc>,

    /// Additional properties that can be set by commands
    pub properties: HashMap<String, String>,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Command-line arguments passed to the application
    pub cli_args: Vec<String>
}

impl CommandContext {
    /// Create a new command context
    pub fn new() -> Self {
        let session_id = uuid::Uuid::new_v4().to_string();
        let user = whoami::username();
        let hostname = whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string());
        let working_directory = std::env::current_dir().unwrap_or_default().to_string_lossy().to_string();
        let timestamp = Utc::now();

        // Collect environment variables
        let env: HashMap<String, String> = std::env::vars().collect();

        // Get CLI arguments
        let cli_args: Vec<String> = std::env::args().collect();

        Self { session_id, user, hostname, working_directory, timestamp, properties: HashMap::new(), env, cli_args }
    }

    /// Set a property in the context
    pub fn set_property(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.properties.insert(key.into(), value.into());
    }

    /// Get a property from the context
    pub fn get_property(&self, key: &str) -> Option<&String> {
        self.properties.get(key)
    }

    /// Get an environment variable
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env.get(key)
    }

    /// Create a child context with additional properties
    pub fn with_properties(&self, properties: HashMap<String, String>) -> Self {
        let mut context = self.clone();
        context.properties.extend(properties);
        context
    }

    /// Create a correlation ID for tracking related operations
    pub fn correlation_id(&self) -> String {
        format!("{}_{}", self.session_id, uuid::Uuid::new_v4())
    }
}

impl Default for CommandContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Workflow information for listing and discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub name:           String,
    pub file_path:      String,
    pub description:    String,
    pub tags:           Vec<String>,
    pub argument_count: usize
}

/// Configuration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInfo {
    pub language:      String,
    pub resource_url:  Option<String>,
    pub workflows_dir: String,
    pub config_dir:    String
}

/// Application state for workflow management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    pub discovered_workflows: Vec<WorkflowInfo>,
    pub selected_workflow:    Option<(String, String)>, // (name, file_path)
    pub execution_history:    Vec<WorkflowExecution>
}

/// Record of a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub workflow_name: String,
    pub command:       String,
    pub timestamp:     DateTime<Utc>,
    pub success:       bool,
    pub user:          String,
    pub hostname:      String
}
