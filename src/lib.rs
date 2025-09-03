//! # Workflow Engine
//!
//! A library for parsing and executing workflow YAML files with interactive argument resolution.
//!
//! This crate provides functionality to:
//! - Parse workflow definitions from YAML files
//! - Resolve arguments interactively (text input, enum selection)
//! - Execute commands with template variable substitution
//! - Provide rich user feedback with progress indicators

// Public API modules
pub mod cli;
pub mod config;
pub mod i18n;
pub mod ui;
pub mod utils;
pub mod workflow;

// Re-export commonly used types
pub use cli::{Cli, Commands, LangCommands, ResourceCommands};
pub use config::{Config, get_config_dir, get_workflows_dir, init_config_dirs};
pub use workflow::{ArgumentType, Workflow, WorkflowArgument};

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_parse_scale_kubernetes_pods_yaml() {
        let yaml_content = fs::read_to_string("resource/scale_kubernetes_pods.yaml").expect("Failed to read YAML file");

        let workflow = Workflow::from_yaml(&yaml_content).expect("Failed to parse YAML");

        // Verify the parsed content
        assert_eq!(workflow.name, "Scale Kubernetes Pods");
        assert_eq!(workflow.description, "Workflow to safely scale down Kubernetes deployments and statefulsets");
        assert_eq!(workflow.arguments.len(), 2);

        // Check namespace argument
        let namespace_arg = &workflow.arguments[0];
        assert_eq!(namespace_arg.name, "namespace");
        assert!(matches!(namespace_arg.arg_type, ArgumentType::Enum));
        assert_eq!(namespace_arg.description, "Namespace to apply scale to");

        assert_eq!(namespace_arg.enum_name.as_ref().unwrap(), "namespaces");
        assert_eq!(namespace_arg.enum_command.as_ref().unwrap(), "kubectl get namespaces | awk 'NR>1 {print $1}'");

        // Check replica_count argument
        let replica_arg = &workflow.arguments[1];
        assert_eq!(replica_arg.name, "replica_count");
        assert!(matches!(replica_arg.arg_type, ArgumentType::Text));
        assert_eq!(replica_arg.description, "Number of replicas");
        assert_eq!(replica_arg.default_value, Some("0".to_string()));

        println!("✅ Successfully parsed workflow: {}", workflow.name);
        println!("Command: {}", workflow.command);
        println!("Arguments: {:?}", workflow.arguments.iter().map(|a| &a.name).collect::<Vec<_>>());
    }
}
