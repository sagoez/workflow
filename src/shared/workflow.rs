//! Core workflow domain types
//!
//! This module contains the core domain types for workflows including
//! workflow definitions, arguments, and related functionality.

use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tera::{Context as TeraContext, Tera};

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

impl Workflow {
    /// Parse a workflow from YAML content
    pub fn from_yaml(yaml_content: &str) -> Result<Self> {
        serde_yaml::from_str(yaml_content).with_context(|| "Failed to parse workflow YAML")
    }

    /// Render the command template by substituting argument values.
    ///
    /// Uses the Tera templating engine to replace {{variable}} placeholders
    /// in the command string with the resolved argument values.
    ///
    /// # Arguments
    /// * `arguments` - HashMap of argument name to resolved value
    ///
    /// # Returns
    /// * `Ok(String)` - The rendered command string
    /// * `Err(anyhow::Error)` - Template rendering error
    ///
    /// # Example
    /// ```rust,no_run
    /// use std::collections::HashMap;
    ///
    /// use workflow::shared::Workflow;
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let workflow = Workflow::from_yaml(
    ///     r#"
    /// name: "Test"
    /// command: "echo {{message}} from {{user}}"
    /// description: "Test workflow"
    /// arguments: []
    /// tags: []
    /// shells: ["bash"]
    /// "#
    /// )?;
    ///
    /// let mut args = HashMap::new();
    /// args.insert("message".to_string(), "Hello".to_string());
    /// args.insert("user".to_string(), "Alice".to_string());
    ///
    /// let command = workflow.render_command(&args)?;
    /// assert_eq!(command, "echo Hello from Alice");
    /// # Ok(())
    /// # }
    /// ```
    pub fn render_command(&self, arguments: &HashMap<String, String>) -> Result<String> {
        let mut tera = Tera::new("templates/*").unwrap_or_else(|_| Tera::new("").unwrap());
        let mut context = TeraContext::new();

        // Add all arguments to the template context
        for (key, value) in arguments {
            context.insert(key, value);
        }

        // Render the command template
        tera.render_str(&self.command, &context).with_context(|| "Failed to render command template")
    }

    /// Get a summary of the workflow for display purposes
    pub fn summary(&self) -> WorkflowSummary {
        WorkflowSummary {
            name:              self.name.clone(),
            description:       self.description.clone(),
            tags:              self.tags.clone(),
            argument_count:    self.arguments.len(),
            has_required_args: self.arguments.iter().any(|arg| arg.default_value.is_none())
        }
    }

    /// Validate that the workflow definition is complete and correct
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            anyhow::bail!("Workflow name cannot be empty");
        }

        if self.command.trim().is_empty() {
            anyhow::bail!("Workflow command cannot be empty");
        }

        if self.description.trim().is_empty() {
            anyhow::bail!("Workflow description cannot be empty");
        }

        // Validate arguments
        for arg in &self.arguments {
            arg.validate()?;
        }

        Ok(())
    }
}

impl WorkflowArgument {
    /// Validate that the argument definition is complete and correct
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            anyhow::bail!("Argument name cannot be empty");
        }

        if self.description.trim().is_empty() {
            anyhow::bail!("Argument description cannot be empty");
        }

        // Validate enum-specific fields
        if matches!(self.arg_type, ArgumentType::Enum) {
            if self.enum_command.is_none() && self.enum_variants.is_none() {
                anyhow::bail!("Enum argument '{}' must have either enum_command or enum_variants", self.name);
            }
        }

        Ok(())
    }

    /// Check if this argument is required (no default value)
    pub fn is_required(&self) -> bool {
        self.default_value.is_none()
    }
}

/// Summary information about a workflow for display
#[derive(Debug, Clone)]
pub struct WorkflowSummary {
    pub name:              String,
    pub description:       String,
    pub tags:              Vec<String>,
    pub argument_count:    usize,
    pub has_required_args: bool
}
