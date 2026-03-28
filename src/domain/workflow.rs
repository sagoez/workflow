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
    pub dynamic_resolution: Option<String>,
    /// For Enum: enable multi-select
    #[serde(default)]
    pub multi:              bool,
    /// For Enum with multi: minimum number of selections
    pub min_selections:     Option<usize>,
    /// For Enum with multi: maximum number of selections
    pub max_selections:     Option<usize>
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
    /// Selection from a list (single or multi based on min/max_selections)
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
            user:              whoami::username().unwrap_or_else(|_| "unknown".to_string()),
            hostname:          whoami::hostname().unwrap_or_else(|_| "unknown".to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_arg_type_is_text() {
        let yaml = r#"
            name: test
            arg_type: Text
            description: "a test"
        "#;
        let arg: WorkflowArgument = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(arg.arg_type, ArgumentType::Text));
    }

    #[test]
    fn arg_type_defaults_to_text_when_omitted() {
        let yaml = r#"
            name: test
            description: "a test"
        "#;
        let arg: WorkflowArgument = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(arg.arg_type, ArgumentType::Text));
    }

    #[test]
    fn all_arg_types_deserialize() {
        for (type_str, expected) in [("Text", "Text"), ("Enum", "Enum"), ("Number", "Number"), ("Boolean", "Boolean")] {
            let yaml = format!("name: test\narg_type: {}\ndescription: a test", type_str);
            let arg: WorkflowArgument = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(format!("{:?}", arg.arg_type), expected);
        }
    }

    #[test]
    fn enum_with_selection_constraints() {
        let yaml = r#"
            name: envs
            arg_type: Enum
            description: "Select envs"
            enum_variants:
              - dev
              - staging
              - prod
            min_selections: 1
            max_selections: 3
        "#;
        let arg: WorkflowArgument = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(arg.arg_type, ArgumentType::Enum));
        assert_eq!(arg.min_selections, Some(1));
        assert_eq!(arg.max_selections, Some(3));
        assert_eq!(arg.enum_variants.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn selection_constraints_default_to_none() {
        let yaml = r#"
            name: test
            description: "a test"
        "#;
        let arg: WorkflowArgument = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(arg.min_selections, None);
        assert_eq!(arg.max_selections, None);
    }

    #[test]
    fn multi_defaults_to_false() {
        let yaml = r#"
            name: env
            arg_type: Enum
            description: "Pick env"
            enum_variants: ["dev", "prod"]
        "#;
        let arg: WorkflowArgument = serde_yaml::from_str(yaml).unwrap();
        assert!(!arg.multi);
    }

    #[test]
    fn multi_true_parses_from_yaml() {
        let yaml = r#"
            name: services
            arg_type: Enum
            multi: true
            description: "Pick services"
            enum_variants: ["api", "web", "worker"]
        "#;
        let arg: WorkflowArgument = serde_yaml::from_str(yaml).unwrap();
        assert!(arg.multi);
        assert!(matches!(arg.arg_type, ArgumentType::Enum));
    }

    #[test]
    fn multi_with_constraints_parses() {
        let yaml = r#"
            name: services
            arg_type: Enum
            multi: true
            description: "Pick services"
            enum_variants: ["api", "web", "worker"]
            min_selections: 1
            max_selections: 2
        "#;
        let arg: WorkflowArgument = serde_yaml::from_str(yaml).unwrap();
        assert!(arg.multi);
        assert_eq!(arg.min_selections, Some(1));
        assert_eq!(arg.max_selections, Some(2));
    }

    #[test]
    fn multi_without_constraints_parses() {
        let yaml = r#"
            name: services
            arg_type: Enum
            multi: true
            description: "Pick services"
            enum_variants: ["api", "web", "worker"]
        "#;
        let arg: WorkflowArgument = serde_yaml::from_str(yaml).unwrap();
        assert!(arg.multi);
        assert_eq!(arg.min_selections, None);
        assert_eq!(arg.max_selections, None);
    }

    #[test]
    fn workflow_deserializes_from_yaml() {
        let yaml = r#"
            name: "Deploy"
            command: "kubectl apply -f {{file}}"
            description: "Deploy to cluster"
            arguments:
              - name: file
                description: "Manifest file"
            tags: ["k8s"]
            shells: ["bash"]
        "#;
        let wf: Workflow = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(wf.name, "Deploy");
        assert_eq!(wf.command, "kubectl apply -f {{file}}");
        assert_eq!(wf.arguments.len(), 1);
        assert_eq!(wf.tags, vec!["k8s"]);
        assert_eq!(wf.shells, vec!["bash"]);
    }

    #[test]
    fn workflow_optional_fields_default_to_none() {
        let yaml = r#"
            name: "Test"
            command: "echo"
            description: "Test"
            arguments: []
            tags: []
            shells: []
        "#;
        let wf: Workflow = serde_yaml::from_str(yaml).unwrap();
        assert!(wf.source_url.is_none());
        assert!(wf.author.is_none());
        assert!(wf.author_url.is_none());
    }

    #[test]
    fn workflow_optional_fields_present() {
        let yaml = r#"
            name: "Test"
            command: "echo"
            description: "Test"
            arguments: []
            tags: []
            shells: []
            source_url: "https://example.com"
            author: "Alice"
            author_url: "https://alice.dev"
        "#;
        let wf: Workflow = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(wf.source_url.as_deref(), Some("https://example.com"));
        assert_eq!(wf.author.as_deref(), Some("Alice"));
        assert_eq!(wf.author_url.as_deref(), Some("https://alice.dev"));
    }

    #[test]
    fn workflow_display_shows_name() {
        let wf = Workflow {
            name:        "My Workflow".to_string(),
            command:     "echo".to_string(),
            description: "desc".to_string(),
            arguments:   vec![],
            tags:        vec![],
            source_url:  None,
            author:      None,
            author_url:  None,
            shells:      vec![]
        };
        assert_eq!(format!("{}", wf), "My Workflow");
    }

    #[test]
    fn workflow_context_new_populates_fields() {
        let ctx = WorkflowContext::new();
        assert!(!ctx.session_id.is_empty());
        assert!(!ctx.user.is_empty());
        assert!(!ctx.hostname.is_empty());
        assert!(!ctx.working_directory.is_empty());
    }

    #[test]
    fn workflow_context_with_session_id_overrides() {
        let ctx = WorkflowContext::with_session_id("custom-session");
        assert_eq!(ctx.session_id, "custom-session");
        assert!(!ctx.user.is_empty());
    }

    #[test]
    fn argument_with_enum_command_and_dynamic_resolution() {
        let yaml = r#"
            name: pod
            arg_type: Enum
            description: "Select pod"
            enum_name: "pods"
            enum_command: "kubectl get pods -n {{namespace}} --no-headers | awk '{print $1}'"
            dynamic_resolution: namespace
        "#;
        let arg: WorkflowArgument = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(arg.enum_name.as_deref(), Some("pods"));
        assert!(arg.enum_command.as_ref().unwrap().contains("{{namespace}}"));
        assert_eq!(arg.dynamic_resolution.as_deref(), Some("namespace"));
    }

    #[test]
    fn argument_default_value_variants() {
        let yaml = "name: x\ndescription: d\ndefault_value: hello";
        let arg: WorkflowArgument = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(arg.default_value.as_deref(), Some("hello"));

        let yaml = "name: x\ndescription: d\ndefault_value: ~";
        let arg: WorkflowArgument = serde_yaml::from_str(yaml).unwrap();
        assert!(arg.default_value.is_none());

        let yaml = "name: x\ndescription: d";
        let arg: WorkflowArgument = serde_yaml::from_str(yaml).unwrap();
        assert!(arg.default_value.is_none());
    }
}
