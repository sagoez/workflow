//! # Workflow CLI Application
//!
//! A command-line interface for executing workflow YAML files with interactive argument resolution.
//!
//! ## Features
//!
//! - **Interactive Workflow Selection**: Run without arguments to choose from available workflows
//! - **Direct Execution**: Specify a workflow file directly as an argument
//! - **Workflow Discovery**: List all available workflows with descriptions
//! - **Smart File Resolution**: Automatically looks in the `resource/` directory
//! - **Rich User Experience**: Progress indicators, spinners, and interactive prompts
//!
//! ## Usage
//!
//! ```bash
//! # Interactive selection from available workflows
//! workflow
//!
//! # Execute a specific workflow
//! workflow "my-workflow.yaml"
//!
//! # List all available workflows
//! workflow --list
//! ```
//!
//! ## Workflow File Location
//!
//! The CLI looks for workflow YAML files in the `resource/` directory relative to the
//! current working directory. Files can have `.yaml` or `.yml` extensions.

use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;
use workflow::{
    cli::{
        Cli, Commands, execute_workflow, handle_init_command, handle_lang_command, handle_resource_command,
        handle_sync_command, list_workflows, select_and_execute_workflow
    },
    config
};

/// Main entry point for the workflow CLI application.
///
/// Parses command-line arguments and dispatches to the appropriate handler:
/// - `--list`: Display all available workflows
/// - `<file>`: Execute a specific workflow file
/// - No arguments: Show interactive workflow selection menu
///
/// # Returns
/// * `Ok(())` - Application completed successfully
/// * `Err(anyhow::Error)` - Application error (file not found, parsing error, etc.)
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle subcommands first
    match &cli.command {
        Some(Commands::Init) => {
            return handle_init_command().await;
        }
        Some(Commands::Lang { command }) => {
            return handle_lang_command(command).await;
        }
        Some(Commands::Resource { command }) => {
            return handle_resource_command(command).await;
        }
        Some(Commands::Sync { url, ssh_key }) => {
            return handle_sync_command(url.as_deref(), ssh_key.as_deref()).await;
        }
        None => {
            // Continue with normal workflow execution logic
        }
    }

    if cli.list {
        return list_workflows().await;
    }

    if let Some(file_path) = &cli.file {
        let full_path = if Path::new(file_path).is_absolute() || file_path.contains('/') {
            file_path.to_string()
        } else {
            // Look in the workflows config directory
            let workflows_dir = config::get_workflows_dir().context("Failed to get workflows directory")?;
            workflows_dir.join(file_path).to_string_lossy().to_string()
        };
        execute_workflow(&full_path).await
    } else {
        select_and_execute_workflow().await
    }
}
