//! Display utilities for formatted output

use crate::{i18n, workflow::Workflow};

/// Display workflow execution header
pub fn show_workflow_header(workflow: &Workflow) {
    if workflow.description.is_empty() || workflow.description == "~" {
        print!("{}", i18n::t_params("execution_workflow_generate_header_no_desc", &[&workflow.name]));
    } else {
        print!("{}", i18n::t_params("execution_workflow_generate_header", &[&workflow.name, &workflow.description]));
    }
}

/// Display selected workflow information
pub fn show_selected_workflow_info(workflow: &Workflow) {
    println!();
    println!("{}", i18n::t_params("cli_selected_workflow", &[&workflow.name]));
    println!("{}", i18n::t_params("cli_workflow_description", &[&workflow.description]));
    println!("{}", i18n::t_params("cli_workflow_arguments", &[&workflow.arguments.len().to_string()]));
}

/// Display final command output
pub fn show_final_command(command: &str) {
    println!();
    println!("$ {}", command);
    println!();
    println!("{}", i18n::t("command_ready_to_execute"));
}

/// Display workflow list item
pub fn show_workflow_list_item(filename: &str, workflow: &Workflow) {
    println!("🔧 {}", filename);
    println!("   Name: {}", workflow.name);
    println!("   Description: {}", workflow.description);
    println!("   Arguments: {}", workflow.arguments.len());
    println!();
}
