//! Core workflow domain types
//!
//! This module contains the core domain types for workflows including
//! workflow definitions, arguments, and related functionality.

use std::{collections::HashMap, fmt::Display, path::PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a complete workflow definition parsed from YAML.
///
/// A workflow contains metadata, a command template with placeholder variables,
/// and a list of arguments that need to be resolved before execution.
///
/// # Example YAML structure
/// ```yaml
/// name: "My Workflow"
/// command: "echo {{message}}"
/// description: "Prints a message"
/// arguments:
///   - name: message
///     arg_type: Text
///     description: "Message to print"
///     default_value: "Hello World"
/// tags: ["example"]
/// shells: ["bash", "zsh"]
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workflow {
    /// Human-readable name of the workflow
    pub name:        String,
    /// Command template with {{variable}} placeholders to be executed
    pub command:     String,
    /// Description explaining what the workflow does
    pub description: String,
    /// List of arguments that need to be resolved before execution
    pub arguments:   Vec<WorkflowArgument>,
    /// Tags for categorizing workflows
    pub tags:        Vec<String>,
    /// Optional URL to the workflow source
    pub source_url:  Option<String>,
    /// Optional workflow author name
    pub author:      Option<String>,
    /// Optional URL to author's profile/website
    pub author_url:  Option<String>,
    /// List of supported shell environments
    pub shells:      Vec<String>
}

impl Display for Workflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Represents a single argument that needs to be resolved before workflow execution.
///
/// Arguments can be of different types and have different resolution mechanisms:
/// - Text/Number/Boolean: User input prompts with optional defaults
/// - Enum: Dynamic options retrieved by executing a command
///
/// # Example YAML structures
/// ```yaml
/// # Simple text argument with default
/// - name: message
///   arg_type: Text  # Optional, defaults to Text
///   description: "Message to display"
///   default_value: "Hello"
///
/// # Enum argument that gets options dynamically
/// - name: namespace
///   arg_type: Enum
///   description: "Kubernetes namespace"
///   enum_name: "namespaces"
///   enum_command: "kubectl get namespaces --no-headers | awk '{print $1}'"
///
/// # Required argument (no default)
/// - name: filename
///   description: "File to process"
///   default_value: ~  # ~ means null/no default
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkflowArgument {
    /// Variable name used in command template (e.g., {{name}})
    pub name:               String,
    /// Type of argument - determines resolution method
    #[serde(default = "default_arg_type")]
    pub arg_type:           ArgumentType,
    /// Human-readable description shown in prompts
    pub description:        String,
    /// Optional default value (use ~ for null/no default)
    pub default_value:      Option<String>,
    /// For Enum type: identifier for the dynamic option set
    pub enum_name:          Option<String>,
    /// For Enum type: command to execute to get available options
    pub enum_command:       Option<String>,
    /// For Enum type: static list of predefined options
    pub enum_variants:      Option<Vec<String>>,
    /// For Enum type: name of the argument to reference for dynamic resolution in enum_command
    pub dynamic_resolution: Option<String>
}

/// Returns the default argument type when not specified in YAML.
fn default_arg_type() -> ArgumentType {
    ArgumentType::Text
}

/// Defines the different types of arguments supported by workflows.
///
/// Each type has a different resolution mechanism:
/// - `Text`: Free text input from user
/// - `Enum`: Selection from dynamically generated options
/// - `Number`: Numeric input (validated as number)
/// - `Boolean`: True/false selection
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum ArgumentType {
    /// Free text input with optional default value
    Text,
    /// Selection from dynamically generated list (via command execution)
    Enum,
    /// Numeric input with validation
    Number,
    /// Boolean true/false selection
    Boolean
}

/// Command execution context
#[derive(Clone, Debug)]
pub struct WorkflowContext {
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

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Command-line arguments passed to the application
    pub cli_args: Vec<String>
}

impl WorkflowContext {
    pub fn new() -> Self {
        Self {
            session_id:        Uuid::new_v4().to_string(),
            user:              whoami::username(),
            hostname:          whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string()),
            working_directory: std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("unknown"))
                .to_string_lossy()
                .to_string(),
            timestamp:         Utc::now(),
            env:               std::env::vars().collect(),
            cli_args:          std::env::args().collect()
        }
    }

    pub fn with_session_id(session_id: &str) -> Self {
        let mut context = Self::new();
        context.session_id = session_id.to_string();
        context
    }
}

impl Default for WorkflowContext {
    fn default() -> Self {
        Self::new()
    }
}
