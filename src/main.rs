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

use clap::{Arg, Command};
use std::fs;
use std::path::Path;
use anyhow::{Result, Context};
use dialoguer::Select;
use workflow::Workflow;

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
    let matches = Command::new("workflow")
        .version("0.1.0")
        .about("Execute workflow YAML files with interactive argument resolution")
        .arg(
            Arg::new("file")
                .help("Path to the workflow YAML file (optional - will show selection menu if not provided)")
                .value_name("FILE")
                .index(1)
        )
        .arg(
            Arg::new("list")
                .long("list")
                .short('l')
                .help("List all available workflows in the resource directory")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    if matches.get_flag("list") {
        return list_workflows().await;
    }

    if let Some(file_path) = matches.get_one::<String>("file") {
        let full_path = if Path::new(file_path).is_absolute() || file_path.contains('/') {
            file_path.to_string()
        } else {
            format!("resource/{}", file_path)
        };
        execute_workflow(&full_path).await
    } else {
        select_and_execute_workflow().await
    }
}

/// Execute a specific workflow file.
///
/// This function handles the complete workflow execution process:
/// 1. Validates the file exists
/// 2. Reads and parses the YAML content
/// 3. Delegates to the workflow's execute method
///
/// # Arguments
/// * `file_path` - Path to the workflow YAML file (can be absolute or relative)
///
/// # Returns
/// * `Ok(())` - Workflow executed successfully
/// * `Err(anyhow::Error)` - File not found, parsing error, or execution failure
///
/// # File Resolution
/// The function accepts both absolute paths and relative paths. If a relative path
/// is provided without directory separators, it automatically looks in the
/// `resource/` directory.
async fn execute_workflow(file_path: &str) -> Result<()> {
    if !Path::new(file_path).exists() {
        anyhow::bail!("Workflow file not found: {}", file_path);
    }

    let yaml_content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read workflow file: {}", file_path))?;

    let workflow = Workflow::from_yaml(&yaml_content)
        .with_context(|| format!("Failed to parse workflow YAML: {}", file_path))?;

    workflow.execute().await
}

/// Display a list of all available workflows in the resource directory.
///
/// This function scans the `resource/` directory for YAML files, attempts to parse
/// each one as a workflow, and displays a formatted list with:
/// - Workflow filename
/// - Workflow name (from YAML metadata)
/// - Description
/// - Number of arguments
///
/// # Returns
/// * `Ok(())` - List displayed successfully (even if no workflows found)
/// * `Err(anyhow::Error)` - Error reading the resource directory
///
/// # Output Format
/// ```text
/// 📋 Available workflows:
///
/// 🔧 example.yaml
///    Name: Example Workflow
///    Description: This is an example workflow
///    Arguments: 2
/// ```
///
/// # Error Handling
/// Invalid YAML files are skipped with a warning message rather than causing
/// the entire operation to fail.
async fn list_workflows() -> Result<()> {
    let resource_dir = "resource";
    
    if !Path::new(resource_dir).exists() {
        println!("📁 No resource directory found. Create 'resource/' and add your workflow YAML files there.");
        return Ok(());
    }

    println!("📋 Available workflows:");
    println!();

    let entries = fs::read_dir(resource_dir)
        .with_context(|| "Failed to read resource directory")?;

    let mut workflows = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") || 
           path.extension().and_then(|s| s.to_str()) == Some("yml") {
            
            let file_name = path.file_name().unwrap().to_str().unwrap();
            
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match Workflow::from_yaml(&content) {
                        Ok(workflow) => {
                            workflows.push((file_name.to_string(), workflow));
                        }
                        Err(_) => {
                            println!("⚠️  {} (invalid YAML)", file_name);
                        }
                    }
                }
                Err(_) => {
                    println!("⚠️  {} (cannot read)", file_name);
                }
            }
        }
    }

    if workflows.is_empty() {
        println!("No valid workflow files found in the resource directory.");
        println!("Add .yaml or .yml files to get started!");
    } else {
        for (filename, workflow) in workflows {
            println!("🔧 {}", filename);
            println!("   Name: {}", workflow.name);
            println!("   Description: {}", workflow.description);
            println!("   Arguments: {}", workflow.arguments.len());
            println!();
        }
        
        println!("Usage:");
        println!("  workflow <filename>           # Execute a workflow");
        println!("  workflow \"Scale Kubernetes Pods.yaml\"  # Example");
    }

    Ok(())
}

/// Present an interactive menu to select and execute a workflow.
///
/// This function provides the main interactive experience when no workflow file
/// is specified on the command line. It:
/// 1. Scans the `resource/` directory for workflow files
/// 2. Parses each file to extract metadata
/// 3. Presents a selection menu with workflow names and descriptions
/// 4. Executes the selected workflow
///
/// # Returns
/// * `Ok(())` - Workflow selected and executed successfully
/// * `Err(anyhow::Error)` - Directory access error, no workflows found, selection error, or execution failure
///
/// # User Experience
/// The selection menu displays workflows in the format:
/// ```text
/// Choose a workflow:
/// > Workflow Name - Brief description of what it does
///   Another Workflow - Another description
/// ```
///
/// Users can navigate with arrow keys and select with Enter.
///
/// # Error Handling
/// - Missing resource directory: Clear error message with setup instructions
/// - No valid workflows: Helpful message about adding YAML files
/// - Invalid YAML files: Skipped with warning messages
/// - User cancellation: Graceful exit
async fn select_and_execute_workflow() -> Result<()> {
    let resource_dir = "resource";
    
    if !Path::new(resource_dir).exists() {
        anyhow::bail!("📁 No resource directory found. Create 'resource/' and add your workflow YAML files there.");
    }

    println!("🔧 Select a workflow to execute:");
    println!();

    let entries = fs::read_dir(resource_dir)
        .with_context(|| "Failed to read resource directory")?;

    let mut workflows = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") || 
           path.extension().and_then(|s| s.to_str()) == Some("yml") {
            
            let file_name = path.file_name().unwrap().to_str().unwrap();
            
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match Workflow::from_yaml(&content) {
                        Ok(workflow) => {
                            workflows.push((file_name.to_string(), workflow, path.to_string_lossy().to_string()));
                        }
                        Err(_) => {
                            println!("⚠️  Skipping {} (invalid YAML)", file_name);
                        }
                    }
                }
                Err(_) => {
                    println!("⚠️  Skipping {} (cannot read)", file_name);
                }
            }
        }
    }

    if workflows.is_empty() {
        anyhow::bail!("No valid workflow files found in the resource directory.\nAdd .yaml or .yml files to get started!");
    }

    let items: Vec<String> = workflows.iter()
        .map(|(_filename, workflow, _)| {
            format!("{} - {}", workflow.name, workflow.description)
        })
        .collect();

    let selection = Select::new()
        .with_prompt("Choose a workflow")
        .items(&items)
        .default(0)
        .interact()
        .with_context(|| "Failed to get workflow selection")?;

    let (_, _, file_path) = &workflows[selection];
    
    println!();
    execute_workflow(file_path).await
}
