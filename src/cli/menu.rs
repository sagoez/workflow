//! Interactive workflow selection and listing functionality

use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::{config, i18n, ui, workflow::Workflow};

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
pub async fn list_workflows() -> Result<()> {
    let workflows_dir = config::get_workflows_dir().context("Failed to get workflows directory")?;

    if !workflows_dir.exists() {
        println!("{}", i18n::t("dir_no_resource_directory"));
        println!("{}", i18n::t("init_tip_run_init"));
        return Ok(());
    }

    println!("📋 {}", i18n::t("cli_available_workflows"));
    println!();

    let entries = fs::read_dir(&workflows_dir).with_context(|| i18n::t("error_failed_to_read_dir"))?;

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
        println!("{}", i18n::t("list_no_workflows"));
    } else {
        for (filename, workflow) in workflows {
            ui::show_workflow_list_item(&filename, &workflow);
        }

        println!("{}", i18n::t("list_usage_header"));
        println!("{}", i18n::t("list_usage_execute"));
        println!("{}", i18n::t("list_usage_example"));
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
pub async fn select_and_execute_workflow() -> Result<()> {
    let workflows_dir = config::get_workflows_dir().context("Failed to get workflows directory")?;

    if !workflows_dir.exists() {
        anyhow::bail!("{}\n{}", i18n::t("dir_no_resource_directory"), i18n::t("init_tip_run_init"));
    }

    let entries = fs::read_dir(&workflows_dir).with_context(|| i18n::t("error_failed_to_read_dir"))?;

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
                        println!("{}", i18n::t_params("file_skipping_invalid_yaml", &[file_name]));
                    }
                },
                Err(_) => {
                    println!("{}", i18n::t_params("file_skipping_cannot_read", &[file_name]));
                }
            }
        }
    }

    if workflows.is_empty() {
        anyhow::bail!(i18n::t("file_no_valid_workflows"));
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

    let selection_index = ui::show_workflow_selection_menu(workflow_names)?;

    let (_, selected_workflow, _) = &workflows[selection_index];
    ui::show_selected_workflow_info(selected_workflow);

    Ok(selection_index)
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
pub async fn execute_workflow(file_path: &str) -> Result<()> {
    if !Path::new(file_path).exists() {
        anyhow::bail!(i18n::t_params("file_workflow_not_found", &[file_path]));
    }

    let yaml_content =
        fs::read_to_string(file_path).with_context(|| i18n::t_params("error_failed_to_read", &[file_path]))?;

    let workflow =
        Workflow::from_yaml(&yaml_content).with_context(|| i18n::t_params("error_failed_to_parse", &[file_path]))?;

    workflow.generate_command().await?;

    Ok(())
}
