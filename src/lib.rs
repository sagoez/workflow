//! # Workflow Engine
//!
//! An event-driven workflow execution engine with interactive argument resolution.
//!
//! This crate provides functionality to:
//! - Parse workflow definitions from YAML files
//! - Execute workflows through an event-driven architecture
//! - Resolve arguments interactively (text input, enum selection)
//! - Track workflow execution history and statistics
//! - Provide rich user feedback with progress indicators
//! - Support multiple languages and Git-based workflow synchronization

// Core modules
pub mod i18n;
pub mod utils;

// Event-driven architecture modules
pub mod adapters;
pub mod ports;
pub mod services;
pub mod shared;

// Re-export commonly used types from the new architecture
// Legacy config functions (temporary - TODO: migrate to ConfigService)
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
// Re-export ports for dependency injection
pub use ports::{
    filesystem::FileSystem,
    git::GitClient,
    network::HttpClient,
    storage::{Config as StorageConfig, ConfigStore, EventStore, HistoryStore},
    ui::{ProgressIndicator, Prompter, Renderer, WorkflowInfo}
};
// Re-export services for direct use
pub use services::{ConfigService, SyncService, WorkflowService};
pub use shared::{
    ArgumentType,
    // Event system
    Command,
    CommandContext,
    ConfigurationInitializedEvent,
    Event,
    LanguageChangedEvent,
    ResourceUrlChangedEvent,
    // Core domain types
    Workflow,
    WorkflowArgument,
    WorkflowCompletedEvent,
    // Events
    WorkflowDiscoveredEvent,
    WorkflowError,
    WorkflowFailedEvent,
    WorkflowSelectedEvent,
    WorkflowStartedEvent,
    WorkflowSummary,
    WorkflowsSyncedEvent
};

/// Get the project directories for cross-platform config path resolution
pub fn get_project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("", "", "workflow-rs").context("Failed to determine project directories")
}

/// Get the configuration directory path
pub fn get_config_dir() -> Result<PathBuf> {
    let project_dirs = get_project_dirs()?;
    Ok(project_dirs.config_dir().to_path_buf())
}

/// Get the workflows directory path
pub fn get_workflows_dir() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("workflows"))
}

/// Initialize configuration directories
pub fn init_config_dirs() -> Result<()> {
    let config_dir = get_config_dir()?;
    let workflows_dir = get_workflows_dir()?;
    let i18n_dir = config_dir.join("i18n");

    std::fs::create_dir_all(&config_dir)?;
    std::fs::create_dir_all(&workflows_dir)?;
    std::fs::create_dir_all(&i18n_dir)?;

    Ok(())
}

// Legacy CLI types for backward compatibility (until fully migrated)
pub use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Path to the workflow YAML file (optional - will show selection menu if not provided)
    #[arg(value_name = "FILE")]
    pub file: Option<String>,

    /// List all available workflows
    #[arg(short, long)]
    pub list: bool,

    #[command(subcommand)]
    pub command: Option<Commands>
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize configuration directories and copy default files
    Init,
    /// Language management commands
    Lang {
        #[command(subcommand)]
        command: LangCommands
    },
    /// Resource management commands
    Resource {
        #[command(subcommand)]
        command: ResourceCommands
    },
    /// Sync workflows by cloning from Git repository
    Sync {
        /// Optional Git URL to use instead of the configured one
        url:     Option<String>,
        /// Path to SSH private key for SSH authentication
        #[arg(long)]
        ssh_key: Option<String>
    }
}

#[derive(Subcommand)]
pub enum LangCommands {
    /// Set the current language
    Set {
        /// Language code (e.g., 'en', 'es')
        language: String
    },
    /// List available languages
    List,
    /// Show current language
    Current
}

#[derive(Subcommand)]
pub enum ResourceCommands {
    /// Set the resource URL for workflows
    Set {
        /// Git URL for workflows repository
        url: String
    },
    /// Show current resource URL
    Current
}

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
