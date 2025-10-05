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
//! - **Actor-based Architecture**: Built on Ractor for robust, fault-tolerant execution
//! - **Event Sourcing**: Commands emit events for reliable state management
//! - **Internationalization**: Multi-language support with translation keys
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
//!
//! # Sync workflows from remote repository
//! workflow sync --ssh-key ~/.ssh/id_rsa --remote-url git@github.com:user/workflows.git
//!
//! # Language management
//! workflow lang set en
//! workflow lang current
//! workflow lang list
//! ```
//!
//! ## Workflow File Location
//!
//! The CLI looks for workflow YAML files in the system configuration directory:
//! - **macOS**: `~/Library/Application Support/workflow/workflows/`
//! - **Linux**: `~/.config/workflow/workflows/`
//! - **Windows**: `%APPDATA%/workflow/workflows/`
//!
//! Files can have `.yaml` or `.yml` extensions. Use the `sync` command to populate
//! this directory from a remote Git repository.

use clap::Parser;
use ractor::{
    ActorRef,
    rpc::{CallResult, call}
};
use workflow::{
    actor::{Guardian, GuardianMessage},
    domain::{
        command::{
            CompleteWorkflowCommand, DiscoverWorkflowsCommand, GetCurrentLanguageCommand, GetCurrentStorageCommand,
            InteractivelySelectWorkflowCommand, LangCommands, ListAggregatesCommand, ListLanguagesCommand,
            ListWorkflowsCommand, PurgeStorageCommand, ReplayAggregateCommand, ResolveArgumentsCommand,
            SetLanguageCommand, SetStorageCommand, StartWorkflowCommand, StorageCommands, SyncWorkflowsCommand,
            WorkflowCli, WorkflowCliCommand, WorkflowCommand
        },
        error::WorkflowError,
        workflow::WorkflowContext
    },
    t, t_params
};

#[tokio::main]
async fn main() -> Result<(), WorkflowError> {
    let guardian_ref = Guardian::spawn_system()
        .await
        .map_err(|e| WorkflowError::Generic(format!("Failed to start actor system: {}", e)))?;

    let cli = WorkflowCli::parse();
    let context = WorkflowContext::new();

    let result = match cli.command {
        Some(WorkflowCliCommand::Sync { ssh_key, remote_url, branch }) => {
            submit_command_to_actor_system(
                &guardian_ref,
                SyncWorkflowsCommand { ssh_key, remote_url, branch }.into(),
                context
            )
            .await
        }
        Some(WorkflowCliCommand::Lang { command }) => match command {
            LangCommands::Set { language } => {
                submit_command_to_actor_system(&guardian_ref, SetLanguageCommand { language }.into(), context).await
            }
            LangCommands::Current => {
                submit_command_to_actor_system(&guardian_ref, GetCurrentLanguageCommand.into(), context).await
            }
            LangCommands::List => {
                submit_command_to_actor_system(&guardian_ref, DiscoverWorkflowsCommand.into(), context.clone()).await?;
                submit_command_to_actor_system(&guardian_ref, ListLanguagesCommand.into(), context).await
            }
        },
        Some(WorkflowCliCommand::Storage { command }) => match command {
            StorageCommands::Set { backend } => {
                submit_command_to_actor_system(&guardian_ref, SetStorageCommand { backend }.into(), context).await
            }
            StorageCommands::Current => {
                submit_command_to_actor_system(&guardian_ref, GetCurrentStorageCommand.into(), context).await
            }
            StorageCommands::List => {
                submit_command_to_actor_system(&guardian_ref, ListAggregatesCommand.into(), context).await
            }
            StorageCommands::Replay { aggregate_id } => {
                submit_command_to_actor_system(&guardian_ref, ReplayAggregateCommand { aggregate_id }.into(), context)
                    .await
            }
            StorageCommands::Purge => {
                submit_command_to_actor_system(&guardian_ref, PurgeStorageCommand.into(), context).await
            }
        },
        Some(WorkflowCliCommand::List) => {
            submit_command_to_actor_system(&guardian_ref, DiscoverWorkflowsCommand.into(), context.clone()).await?;
            submit_command_to_actor_system(&guardian_ref, ListWorkflowsCommand.into(), context).await
        }
        Some(WorkflowCliCommand::File { .. }) => {
            println!("{}", t!("error_file_workflow_execution_not_yet_implemented_in_actor_system"));
            Ok(())
        }
        None => {
            submit_command_to_actor_system(&guardian_ref, DiscoverWorkflowsCommand.into(), context.clone()).await?;
            submit_command_to_actor_system(&guardian_ref, InteractivelySelectWorkflowCommand.into(), context.clone())
                .await?;
            submit_command_to_actor_system(&guardian_ref, StartWorkflowCommand.into(), context.clone()).await?;
            submit_command_to_actor_system(&guardian_ref, ResolveArgumentsCommand.into(), context.clone()).await?;
            submit_command_to_actor_system(&guardian_ref, CompleteWorkflowCommand.into(), context.clone()).await?;

            Ok(())
        }
    };

    if let Err(e) = guardian_ref.cast(workflow::actor::GuardianMessage::Shutdown) {
        eprintln!("{}", t_params!("error_failed_to_shutdown_actor_system", &[&format!("{:?}", e)]));
    }

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    result
}

/// Submit a command to the actor system via the Guardian
async fn submit_command_to_actor_system(
    guardian_ref: &ActorRef<GuardianMessage>,
    command: WorkflowCommand,
    context: WorkflowContext
) -> Result<(), WorkflowError> {
    match call(
        guardian_ref,
        |reply| GuardianMessage::SubmitCommand { command, context: Box::new(context), reply },
        Some(std::time::Duration::from_secs(30))
    )
    .await
    {
        Ok(CallResult::Success(Ok(()))) => Ok(()),
        Ok(CallResult::Success(Err(e))) => Err(e),
        Ok(CallResult::Timeout) => Err(WorkflowError::Generic(t!("error_command_processing_timed_out"))),
        Ok(_) => Err(WorkflowError::Generic(t!("error_failed_to_send_command_to_actor_system"))),
        Err(e) => {
            let workflow_error = match e {
                ractor::MessagingErr::SendErr(_) => {
                    WorkflowError::Generic(t!("error_failed_to_send_command_to_actor_system"))
                }
                ractor::MessagingErr::ChannelClosed => WorkflowError::Generic(t!("error_workflow_manager_call_failed")),
                ractor::MessagingErr::InvalidActorType => {
                    WorkflowError::Generic(t!("error_actor_system_not_initialized"))
                }
            };
            Err(workflow_error)
        }
    }
}
