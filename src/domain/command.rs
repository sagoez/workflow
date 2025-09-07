//! Command execution context providing dependencies and environment

use std::{collections::HashMap, fmt::Debug};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::domain::workflow::Workflow;

/// Main CLI application
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct WorkflowCli {
    /// Subcommands
    #[command(subcommand)]
    pub command: Option<WorkflowCliCommand>
}

/// Available CLI commands
#[derive(Subcommand, Debug)]
pub enum WorkflowCliCommand {
    /// Sync workflows from remote git repository
    Sync {
        /// SSH key path for authentication
        #[arg(long)]
        ssh_key:    Option<String>,
        /// Remote repository URL (optional, uses configured remote)
        #[arg(long)]
        remote_url: Option<String>,
        /// Branch to sync (defaults to main)
        #[arg(long, default_value = "main")]
        branch:     String
    },
    /// Language management commands
    Lang {
        #[command(subcommand)]
        command: LangCommands
    },
    /// List available workflows
    List,
    /// Select a workflow
    File {
        /// Path to the workflow file
        file: String
    }
}

/// Language management subcommands
#[derive(Subcommand, Debug)]
pub enum LangCommands {
    /// Set the current language
    Set {
        /// Language code (e.g., 'en', 'es')
        language: String
    },
    /// Show current language
    Current,
    /// List available languages
    List
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiscoverWorkflowsCommand;

#[derive(Debug, Clone)]
pub struct DiscoverWorkflowsData {
    pub workflows: Vec<Workflow>
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InteractivelySelectWorkflowCommand;

pub struct InteractivelySelectWorkflowData {
    pub workflow: Workflow
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StartWorkflowCommand;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResolveArgumentsCommand;

#[derive(Debug, Clone)]
pub struct ResolveArgumentsData {
    pub workflow:           Workflow,
    pub resolved_arguments: HashMap<String, String>
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ListWorkflowsCommand;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompleteWorkflowCommand;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SyncWorkflowsCommand {
    pub ssh_key:    Option<String>,
    pub remote_url: Option<String>,
    pub branch:     String
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecordSyncResultCommand {
    pub commit_id: String
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SetLanguageCommand {
    pub language: String
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GetCurrentLanguageCommand;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ListLanguagesCommand;

// **********************
// Workflow Commands - All commands in the system
// **********************

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowCommand {
    // Workflow management
    DiscoverWorkflows(DiscoverWorkflowsCommand),
    ListWorkflows(ListWorkflowsCommand),
    InteractivelySelectWorkflow(InteractivelySelectWorkflowCommand),
    StartWorkflow(StartWorkflowCommand),
    CompleteWorkflow(CompleteWorkflowCommand),
    ResolveArguments(ResolveArgumentsCommand),

    // Sync operations
    SyncWorkflows(SyncWorkflowsCommand),
    RecordSyncResult(RecordSyncResultCommand),

    // Language management
    SetLanguage(SetLanguageCommand),
    GetCurrentLanguage(GetCurrentLanguageCommand),
    ListLanguages(ListLanguagesCommand)
}

impl Into<WorkflowCommand> for SyncWorkflowsCommand {
    fn into(self) -> WorkflowCommand {
        WorkflowCommand::SyncWorkflows(self)
    }
}

impl Into<WorkflowCommand> for SetLanguageCommand {
    fn into(self) -> WorkflowCommand {
        WorkflowCommand::SetLanguage(self)
    }
}

impl Into<WorkflowCommand> for GetCurrentLanguageCommand {
    fn into(self) -> WorkflowCommand {
        WorkflowCommand::GetCurrentLanguage(self)
    }
}

impl Into<WorkflowCommand> for ListLanguagesCommand {
    fn into(self) -> WorkflowCommand {
        WorkflowCommand::ListLanguages(self)
    }
}

impl Into<WorkflowCommand> for DiscoverWorkflowsCommand {
    fn into(self) -> WorkflowCommand {
        WorkflowCommand::DiscoverWorkflows(self)
    }
}

impl Into<WorkflowCommand> for ListWorkflowsCommand {
    fn into(self) -> WorkflowCommand {
        WorkflowCommand::ListWorkflows(self)
    }
}

impl Into<WorkflowCommand> for InteractivelySelectWorkflowCommand {
    fn into(self) -> WorkflowCommand {
        WorkflowCommand::InteractivelySelectWorkflow(self)
    }
}

impl Into<WorkflowCommand> for StartWorkflowCommand {
    fn into(self) -> WorkflowCommand {
        WorkflowCommand::StartWorkflow(self)
    }
}

impl Into<WorkflowCommand> for RecordSyncResultCommand {
    fn into(self) -> WorkflowCommand {
        WorkflowCommand::RecordSyncResult(self)
    }
}

impl Into<WorkflowCommand> for ResolveArgumentsCommand {
    fn into(self) -> WorkflowCommand {
        WorkflowCommand::ResolveArguments(self)
    }
}

impl Into<WorkflowCommand> for CompleteWorkflowCommand {
    fn into(self) -> WorkflowCommand {
        WorkflowCommand::CompleteWorkflow(self)
    }
}
