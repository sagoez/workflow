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

use clap::Parser;
use ractor::{
    ActorRef,
    rpc::{CallResult, call}
};
use workflow::{
    actor::{Guardian, GuardianMessage},
    domain::{
        command::{
            CompleteWorkflowCommand, DiscoverWorkflowsCommand, GetCurrentLanguageCommand,
            InteractivelySelectWorkflowCommand, LangCommands, ListLanguagesCommand, ListWorkflowsCommand,
            ResolveArgumentsCommand, SetLanguageCommand, StartWorkflowCommand, SyncWorkflowsCommand, WorkflowCli,
            WorkflowCliCommand, WorkflowCommand
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
                submit_command_to_actor_system(&guardian_ref, ListLanguagesCommand.into(), context).await
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
        |reply| GuardianMessage::SubmitCommand { command, context, reply },
        Some(std::time::Duration::from_secs(30))
    )
    .await
    {
        Ok(CallResult::Success(Ok(()))) => Ok(()),
        Ok(CallResult::Success(Err(e))) => {
            Err(WorkflowError::Generic(t_params!("error_command_processing_failed", &[&format!("{:?}", e)])))
        }
        Ok(CallResult::Timeout) => Err(WorkflowError::Generic(t!("error_command_processing_timed_out"))),
        Ok(_) => Err(WorkflowError::Generic(t!("error_failed_to_send_command_to_actor_system"))),
        Err(e) => Err(WorkflowError::Generic(t_params!("error_failed_to_submit_command", &[&format!("{:?}", e)])))
    }
}
