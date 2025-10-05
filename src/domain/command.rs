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
    /// Storage backend management commands
    Storage {
        #[command(subcommand)]
        command: StorageCommands
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

/// Storage management subcommands
#[derive(Subcommand, Debug)]
pub enum StorageCommands {
    /// Set the storage backend
    Set {
        /// Storage backend
        backend: crate::adapter::storage::EventStoreType
    },
    /// Show current storage backend
    Current,
    /// List all aggregate IDs
    List,
    /// Replay events for a specific aggregate ID
    Replay {
        /// Aggregate ID to replay
        aggregate_id: String
    },
    /// Purge all data from storage
    Purge
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SetStorageCommand {
    pub backend: crate::adapter::storage::EventStoreType
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GetCurrentStorageCommand;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ListAggregatesCommand;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplayAggregateCommand {
    pub aggregate_id: String
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PurgeStorageCommand;

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
    ListLanguages(ListLanguagesCommand),

    // Storage management
    SetStorage(SetStorageCommand),
    GetCurrentStorage(GetCurrentStorageCommand),
    ListAggregates(ListAggregatesCommand),
    ReplayAggregate(ReplayAggregateCommand),
    PurgeStorage(PurgeStorageCommand)
}

impl From<SyncWorkflowsCommand> for WorkflowCommand {
    fn from(val: SyncWorkflowsCommand) -> Self {
        WorkflowCommand::SyncWorkflows(val)
    }
}

impl From<SetLanguageCommand> for WorkflowCommand {
    fn from(val: SetLanguageCommand) -> Self {
        WorkflowCommand::SetLanguage(val)
    }
}

impl From<GetCurrentLanguageCommand> for WorkflowCommand {
    fn from(val: GetCurrentLanguageCommand) -> Self {
        WorkflowCommand::GetCurrentLanguage(val)
    }
}

impl From<ListLanguagesCommand> for WorkflowCommand {
    fn from(val: ListLanguagesCommand) -> Self {
        WorkflowCommand::ListLanguages(val)
    }
}

impl From<SetStorageCommand> for WorkflowCommand {
    fn from(val: SetStorageCommand) -> Self {
        WorkflowCommand::SetStorage(val)
    }
}

impl From<GetCurrentStorageCommand> for WorkflowCommand {
    fn from(val: GetCurrentStorageCommand) -> Self {
        WorkflowCommand::GetCurrentStorage(val)
    }
}

impl From<ListAggregatesCommand> for WorkflowCommand {
    fn from(val: ListAggregatesCommand) -> Self {
        WorkflowCommand::ListAggregates(val)
    }
}

impl From<ReplayAggregateCommand> for WorkflowCommand {
    fn from(val: ReplayAggregateCommand) -> Self {
        WorkflowCommand::ReplayAggregate(val)
    }
}

impl From<PurgeStorageCommand> for WorkflowCommand {
    fn from(val: PurgeStorageCommand) -> Self {
        WorkflowCommand::PurgeStorage(val)
    }
}

impl From<DiscoverWorkflowsCommand> for WorkflowCommand {
    fn from(val: DiscoverWorkflowsCommand) -> Self {
        WorkflowCommand::DiscoverWorkflows(val)
    }
}

impl From<ListWorkflowsCommand> for WorkflowCommand {
    fn from(val: ListWorkflowsCommand) -> Self {
        WorkflowCommand::ListWorkflows(val)
    }
}

impl From<InteractivelySelectWorkflowCommand> for WorkflowCommand {
    fn from(val: InteractivelySelectWorkflowCommand) -> Self {
        WorkflowCommand::InteractivelySelectWorkflow(val)
    }
}

impl From<StartWorkflowCommand> for WorkflowCommand {
    fn from(val: StartWorkflowCommand) -> Self {
        WorkflowCommand::StartWorkflow(val)
    }
}

impl From<RecordSyncResultCommand> for WorkflowCommand {
    fn from(val: RecordSyncResultCommand) -> Self {
        WorkflowCommand::RecordSyncResult(val)
    }
}

impl From<ResolveArgumentsCommand> for WorkflowCommand {
    fn from(val: ResolveArgumentsCommand) -> Self {
        WorkflowCommand::ResolveArguments(val)
    }
}

impl From<CompleteWorkflowCommand> for WorkflowCommand {
    fn from(val: CompleteWorkflowCommand) -> Self {
        WorkflowCommand::CompleteWorkflow(val)
    }
}
