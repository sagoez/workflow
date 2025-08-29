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

use std::{fs, path::Path};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use inquire::Select;
use workflow::{Workflow, config, text};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Path to the workflow YAML file (optional - will show selection menu if not provided)
    #[arg(value_name = "FILE")]
    file: Option<String>,

    /// List all available workflows
    #[arg(short, long)]
    list: bool,

    #[command(subcommand)]
    command: Option<Commands>
}

#[derive(Subcommand)]
enum Commands {
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
enum LangCommands {
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
enum ResourceCommands {
    /// Set the resource URL for workflows
    Set {
        /// Git URL for workflows repository
        url: String
    },
    /// Show current resource URL
    Current
}

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
        anyhow::bail!(text::t_params("file_workflow_not_found", &[file_path]));
    }

    let yaml_content =
        fs::read_to_string(file_path).with_context(|| text::t_params("error_failed_to_read", &[file_path]))?;

    let workflow =
        Workflow::from_yaml(&yaml_content).with_context(|| text::t_params("error_failed_to_parse", &[file_path]))?;

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
    let workflows_dir = config::get_workflows_dir().context("Failed to get workflows directory")?;

    if !workflows_dir.exists() {
        println!("{}", text::t("dir_no_resource_directory"));
        println!("{}", text::t("init_tip_run_init"));
        return Ok(());
    }

    println!("📋 {}", text::t("cli_available_workflows"));
    println!();

    let entries = fs::read_dir(&workflows_dir).with_context(|| text::t("error_failed_to_read_dir"))?;

    let mut workflows = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            let file_name = path
                .file_name()
                .ok_or(anyhow::anyhow!("Failed to get file name"))?
                .to_str()
                .ok_or(anyhow::anyhow!("Failed to get file name"))?;

            // Skip config.yaml as it's not a workflow file
            if file_name == "config.yaml" {
                continue;
            }

            match fs::read_to_string(&path) {
                Ok(content) => match Workflow::from_yaml(&content) {
                    Ok(workflow) => {
                        workflows.push((file_name.to_string(), workflow));
                    }
                    Err(_) => {
                        println!("⚠️  {} (invalid YAML)", file_name);
                    }
                },
                Err(_) => {
                    println!("⚠️  {} (cannot read)", file_name);
                }
            }
        }
    }

    if workflows.is_empty() {
        println!("{}", text::t("list_no_workflows"));
    } else {
        for (filename, workflow) in workflows {
            println!("🔧 {}", filename);
            println!("   Name: {}", workflow.name);
            println!("   Description: {}", workflow.description);
            println!("   Arguments: {}", workflow.arguments.len());
            println!();
        }

        println!("{}", text::t("list_usage_header"));
        println!("{}", text::t("list_usage_execute"));
        println!("{}", text::t("list_usage_example"));
    }

    Ok(())
}

/// Handle resource commands
async fn handle_resource_command(command: &ResourceCommands) -> Result<()> {
    match command {
        ResourceCommands::Set { url } => {
            config::set_resource_url(url).context("Failed to set resource URL")?;

            println!("{}", text::t_params("resource_set_success", &[url]));
            println!("{}", text::t("resource_set_tip"));
        }
        ResourceCommands::Current => {
            let current = config::get_current_resource_url().context("Failed to get current resource URL")?;

            match current {
                Some(url) => println!("{}", text::t_params("resource_current_url", &[&url])),
                None => println!("{}", text::t("resource_no_url"))
            }
        }
    }

    Ok(())
}

/// Handle sync command
async fn handle_sync_command(url: Option<&str>, ssh_key: Option<&str>) -> Result<()> {
    let workflows_dir = config::get_workflows_dir().context("Failed to get workflows directory")?;

    let resource_url = match url {
        Some(url) => url.to_string(),
        None => {
            let configured_url = config::get_current_resource_url().context("Failed to get current resource URL")?;

            match configured_url {
                Some(url) => url,
                None => {
                    anyhow::bail!("{}", text::t("sync_no_url_configured"));
                }
            }
        }
    };

    config::clone_workflows_from_git(&workflows_dir, &resource_url, ssh_key.as_deref())?;

    println!("{}", text::t_params("sync_success", &[&resource_url]));
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
/// * `Err(anyhow::Error)` - Directory access error, no workflows found, selection error, or
///   execution failure
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
    let workflows_dir = config::get_workflows_dir().context("Failed to get workflows directory")?;

    if !workflows_dir.exists() {
        anyhow::bail!("{}\n{}", text::t("dir_no_resource_directory"), text::t("init_tip_run_init"));
    }

    let entries = fs::read_dir(&workflows_dir).with_context(|| text::t("error_failed_to_read_dir"))?;

    let mut workflows = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            let file_name = path
                .file_name()
                .ok_or(anyhow::anyhow!("Failed to get file name"))?
                .to_str()
                .ok_or(anyhow::anyhow!("Failed to get file name"))?;

            // Skip config.yaml as it's not a workflow file
            if file_name == "config.yaml" {
                continue;
            }

            match fs::read_to_string(&path) {
                Ok(content) => match Workflow::from_yaml(&content) {
                    Ok(workflow) => {
                        workflows.push((file_name.to_string(), workflow, path.to_string_lossy().to_string()));
                    }
                    Err(_) => {
                        println!("{}", text::t_params("file_skipping_invalid_yaml", &[file_name]));
                    }
                },
                Err(_) => {
                    println!("{}", text::t_params("file_skipping_cannot_read", &[file_name]));
                }
            }
        }
    }

    if workflows.is_empty() {
        anyhow::bail!(text::t("file_no_valid_workflows"));
    }

    // Sort workflows by name for better organization
    workflows.sort_by(|a, b| a.1.name.cmp(&b.1.name));

    let selection = show_interactive_workflow_menu(&workflows)?;

    let (_, _, file_path) = &workflows[selection];

    println!();
    execute_workflow(file_path).await
}

/// Show an interactive workflow selection menu with arrow key navigation and toggle descriptions
fn show_interactive_workflow_menu(workflows: &[(String, Workflow, String)]) -> Result<usize> {
    let workflow_names: Vec<String> =
        workflows.iter().map(|(_, workflow, _)| format!("🔧 {}", workflow.name)).collect();

    let selection = Select::new(&text::t("cli_select_prompt"), workflow_names)
        .with_page_size(10)
        .prompt()
        .with_context(|| "Failed to show workflow selection")?;

    let selection_index =
        workflows.iter().position(|(_, workflow, _)| format!("🔧 {}", workflow.name) == selection).unwrap_or(0);

    let (_, selected_workflow, _) = &workflows[selection_index];
    println!();
    println!("{}", text::t_params("cli_selected_workflow", &[&selected_workflow.name]));
    println!("{}", text::t_params("cli_workflow_description", &[&selected_workflow.description]));
    println!("{}", text::t_params("cli_workflow_arguments", &[&selected_workflow.arguments.len().to_string()]));

    Ok(selection_index)
}

/// Handle the init command - initialize configuration directories
async fn handle_init_command() -> Result<()> {
    println!("{}", text::t("init_initializing"));

    config::init_config_dirs().context("Failed to initialize configuration directories")?;

    let config_dir = config::get_config_dir()?;
    let workflows_dir = config::get_workflows_dir()?;
    let i18n_dir = config::get_i18n_dir()?;

    println!("{}", text::t("init_success"));
    println!("{}", text::t_params("init_config_dir", &[&config_dir.display().to_string()]));
    println!("{}", text::t_params("init_workflows_dir", &[&workflows_dir.display().to_string()]));
    println!("{}", text::t_params("init_i18n_dir", &[&i18n_dir.display().to_string()]));
    println!();
    println!("{}", text::t("init_instructions_header"));
    println!("{}", text::t_params("init_instructions_workflows", &[&workflows_dir.display().to_string()]));
    println!("{}", text::t_params("init_instructions_translations", &[&i18n_dir.display().to_string()]));
    println!("{}", text::t("init_instructions_language"));

    Ok(())
}

/// Handle language commands
async fn handle_lang_command(command: &LangCommands) -> Result<()> {
    match command {
        LangCommands::Set { language } => {
            // Validate that the language exists
            let available_languages =
                config::list_available_languages().context("Failed to list available languages")?;

            if !available_languages.contains(language) {
                anyhow::bail!(text::t_params("lang_unknown_language", &[language, &available_languages.join(", ")]));
            }

            config::set_language(language).context("Failed to set language")?;

            println!("{}", text::t_params("lang_set_success", &[language]));
        }
        LangCommands::List => {
            let languages = config::list_available_languages().context("Failed to list available languages")?;

            let current = config::get_current_language().unwrap_or_else(|_| "en".to_string());

            println!("{}", text::t("lang_available_header"));
            for lang in languages {
                let marker = if lang == current { text::t("lang_current_marker") } else { "".to_string() };
                println!("  • {}{}", lang, marker);
            }
        }
        LangCommands::Current => {
            let current = config::get_current_language().context("Failed to get current language")?;
            println!("{}", text::t_params("lang_current_language", &[&current]));
        }
    }

    Ok(())
}
