//! WorkflowManager Actor - Manages workflow sessions and command routing
//!
//! The WorkflowManager acts as a coordinator for all workflow sessions
//! - Coordinates all customer sessions (workflow executions)
//! - Spawns and manages CommandProcessor actors per session
//! - Routes commands to the appropriate session processors
//! - Handles session lifecycle (creation, completion, failure)
//! - Provides system-wide session statistics

use std::{collections::HashMap, sync::Arc};

use ractor::{
    Actor, ActorProcessingErr, ActorRef, SpawnErr,
    rpc::{CallResult, call}
};
use tracing::{Level, event};

use crate::{
    AppContext,
    actor::{
        message::{CommandProcessorMessage, WorkflowManagerMessage},
        processor::CommandProcessor
    },
    adapter::{
        engine::EngineFactory,
        journal::{JournalFactory, JournalType},
        storage::EventStoreType
    },
    domain::{command::WorkflowCommand, constant::workflow_manager, error::WorkflowError, workflow::WorkflowContext},
    port::command::Command,
    t_params
};

/// WorkflowManager Actor State - tracks all active sessions and statistics
pub struct WorkflowManagerState {
    /// Active session processors (session_id -> processor_ref)
    active_sessions:          HashMap<String, ActorRef<CommandProcessorMessage>>,
    /// Shared application context
    app_context:              Arc<AppContext>,
    /// Statistics for monitoring and health checks
    total_sessions_created:   u64,
    /// Total commands processed
    total_commands_processed: u64,
    /// Total sessions failed
    total_sessions_failed:    u64
}

/// WorkflowManager Actor - Manages workflow sessions
/// This is like the "Branch Manager" - coordinates all customer sessions
pub struct WorkflowManager;

#[async_trait::async_trait]
impl Actor for WorkflowManager {
    type Arguments = Arc<AppContext>;
    type Msg = WorkflowManagerMessage;
    type State = WorkflowManagerState;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        app_context: Self::Arguments
    ) -> Result<Self::State, ActorProcessingErr> {
        event!(Level::DEBUG, event = workflow_manager::MANAGER_STARTED);

        Ok(WorkflowManagerState {
            active_sessions: HashMap::new(),
            app_context,
            total_sessions_created: 0,
            total_commands_processed: 0,
            total_sessions_failed: 0
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State
    ) -> Result<(), ActorProcessingErr> {
        match message {
            WorkflowManagerMessage::SubmitCommand { command, context, reply } => {
                let result = self.handle_submit_command(myself, command, *context, state).await;
                let response = match &result {
                    Ok(_) => Ok(()),
                    Err(e) => Err(WorkflowError::Execution(e.to_string()))
                };
                if let Err(e) = reply.send(response) {
                    event!(Level::ERROR, event = workflow_manager::COMMAND_SUBMITTED, error = %e);
                }
                result
            }
            WorkflowManagerMessage::SessionCompleted { session_id } => {
                self.handle_session_completed(session_id, state).await
            }
            WorkflowManagerMessage::SessionFailed { session_id, error } => {
                self.handle_session_failed(session_id, error, state).await
            }
            WorkflowManagerMessage::GetActiveSessions { reply } => {
                if let Err(e) = reply.send(state.active_sessions.len()) {
                    event!(Level::ERROR, event = workflow_manager::MANAGER_STARTED, error = %e);
                }
                Ok(())
            }
        }
    }
}

impl WorkflowManager {
    async fn handle_submit_command(
        &self,
        _myself: ActorRef<WorkflowManagerMessage>,
        command: WorkflowCommand,
        context: WorkflowContext,
        state: &mut WorkflowManagerState
    ) -> Result<(), ActorProcessingErr> {
        let session_id = context.session_id.clone();

        event!(Level::DEBUG, event = workflow_manager::COMMAND_SUBMITTED,
               session_id = %session_id, command = %command.name());

        let processor_ref = match state.active_sessions.get(&session_id) {
            Some(existing_processor) => {
                event!(Level::DEBUG, event = workflow_manager::COMMAND_SUBMITTED,
                       session_id = %session_id, message = "using_existing_processor");
                existing_processor.clone()
            }
            None => match self.spawn_command_processor(&session_id, state.app_context.clone()).await {
                Ok(processor_ref) => {
                    state.active_sessions.insert(session_id.clone(), processor_ref.clone());
                    state.total_sessions_created += 1;

                    event!(Level::DEBUG, event = workflow_manager::PROCESSOR_SPAWNED,
                               session_id = %session_id,
                               total_sessions = %state.total_sessions_created);

                    processor_ref
                }
                Err(e) => {
                    event!(Level::ERROR, event = workflow_manager::PROCESSOR_SPAWN_FAILED,
                               session_id = %session_id, error = %e);
                    return Err(ActorProcessingErr::from(t_params!(
                        "error_failed_to_spawn_command_processor",
                        &[&e.to_string()]
                    )));
                }
            }
        };

        // Use call to get the actual command processing result
        match call(
            &processor_ref,
            |reply| CommandProcessorMessage::ProcessCommand { command: command.clone(), reply },
            Some(std::time::Duration::from_secs(30))
        )
        .await
        {
            Ok(CallResult::Success(Ok(()))) => {
                // Command processed successfully
                event!(Level::DEBUG, event = workflow_manager::COMMAND_SUBMITTED,
                       session_id = %session_id, message = "command_processed_successfully");
            }
            Ok(CallResult::Success(Err(e))) => {
                // Command processing failed - this is the real error
                event!(Level::ERROR, event = workflow_manager::COMMAND_SUBMITTED,
                       session_id = %session_id, error = ?e, message = "command_processing_failed");

                // Return a clean error without wrapping
                return Err(e);
            }
            Ok(CallResult::Timeout) => {
                // Call timed out
                event!(Level::ERROR, event = workflow_manager::COMMAND_SUBMITTED,
                       session_id = %session_id, message = "call_timeout");
                return Err(ActorProcessingErr::from(t_params!("error_command_timeout", &[&"30 seconds"])));
            }
            Ok(CallResult::SenderError) => {
                // Sender error
                event!(Level::ERROR, event = workflow_manager::COMMAND_SUBMITTED,
                       session_id = %session_id, message = "sender_error");
                return Err(ActorProcessingErr::from(t_params!(
                    "error_failed_to_send_command_to_processor",
                    &[&"Sender error"]
                )));
            }
            Err(e) => {
                // Failed to send command to processor (actor is dead)
                event!(Level::ERROR, event = workflow_manager::COMMAND_SUBMITTED,
                       session_id = %session_id, error = ?e, message = "failed_to_send_command");
                return Err(ActorProcessingErr::from(t_params!(
                    "error_failed_to_send_command_to_processor",
                    &[&format!("{:?}", e)]
                )));
            }
        }

        state.total_commands_processed += 1;
        event!(Level::DEBUG, event = workflow_manager::COMMAND_SUBMITTED,
               session_id = %session_id,
               total_processed = %state.total_commands_processed);

        Ok(())
    }

    async fn spawn_command_processor(
        &self,
        session_id: &str,
        app_context: Arc<AppContext>
    ) -> Result<ActorRef<CommandProcessorMessage>, SpawnErr> {
        // Create engine and journal for this session
        let engine = EngineFactory::init(EventStoreType::InMemory, (*app_context).clone());
        let journal = JournalFactory::create(JournalType::InMemory);

        let processor_name = format!("command_processor_{}", session_id);
        let (processor_ref, _handle) = Actor::spawn(
            Some(processor_name),
            CommandProcessor,
            (session_id.to_string(), engine, journal, app_context)
        )
        .await?;

        Ok(processor_ref)
    }

    async fn handle_session_completed(
        &self,
        session_id: String,
        state: &mut WorkflowManagerState
    ) -> Result<(), ActorProcessingErr> {
        event!(Level::DEBUG, event = workflow_manager::SESSION_COMPLETED, session_id = %session_id);

        if let Some(processor) = state.active_sessions.remove(&session_id) {
            processor.stop(None);
        }

        Ok(())
    }

    async fn handle_session_failed(
        &self,
        session_id: String,
        error: String,
        state: &mut WorkflowManagerState
    ) -> Result<(), ActorProcessingErr> {
        event!(Level::ERROR, event = workflow_manager::SESSION_FAILED,
               session_id = %session_id, error = %error);

        if let Some(processor) = state.active_sessions.remove(&session_id) {
            processor.stop(None);
            event!(Level::DEBUG, event = workflow_manager::SESSION_FAILED,
                   session_id = %session_id, message = "processor_stopped");
        }

        state.total_sessions_failed += 1;

        self.handle_session_failure_recovery(&session_id, &error, state).await?;

        Ok(())
    }

    async fn handle_session_failure_recovery(
        &self,
        session_id: &str,
        error: &str,
        state: &mut WorkflowManagerState
    ) -> Result<(), ActorProcessingErr> {
        let is_recoverable = self.is_recoverable_failure(error);

        if is_recoverable {
            event!(Level::WARN, event = workflow_manager::SESSION_FAILED,
                   session_id = %session_id,
                   message = "recoverable_failure_detected",
                   recovery_action = "restart_available");

            // For now, just log that restart is available
            // In a full implementation, we might:
            // 1. Attempt to restart the session
            // 2. Replay failed commands from event store
            // 3. Notify monitoring systems
        } else {
            event!(Level::ERROR, event = workflow_manager::SESSION_FAILED,
                   session_id = %session_id,
                   message = "unrecoverable_failure",
                   total_failed = %state.total_sessions_failed);

            // For unrecoverable failures:
            // 1. Alert operations team
            // 2. Mark session as permanently failed
            // 3. Clean up resources
        }

        Ok(())
    }

    /// Determine if a failure is recoverable based on error message
    fn is_recoverable_failure(&self, error: &str) -> bool {
        // Simple heuristics for recoverable vs unrecoverable failures
        let recoverable_patterns = ["timeout", "connection", "network", "temporary", "retry"];

        let unrecoverable_patterns = ["validation", "authentication", "authorization", "parse", "format"];

        let error_lower = error.to_lowercase();

        if unrecoverable_patterns.iter().any(|pattern| error_lower.contains(pattern)) {
            return false;
        }

        recoverable_patterns.iter().any(|pattern| error_lower.contains(pattern))
    }
}

impl WorkflowManager {
    /// Get session statistics for monitoring
    pub fn get_session_stats(state: &WorkflowManagerState) -> SessionStats {
        SessionStats {
            active_sessions:          state.active_sessions.len(),
            total_sessions_created:   state.total_sessions_created,
            total_commands_processed: state.total_commands_processed,
            total_sessions_failed:    state.total_sessions_failed,
            success_rate:             if state.total_sessions_created > 0 {
                ((state.total_sessions_created - state.total_sessions_failed) as f64
                    / state.total_sessions_created as f64)
                    * 100.0
            } else {
                100.0
            }
        }
    }
}

/// Session statistics for monitoring and health checks
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub active_sessions:          usize,
    pub total_sessions_created:   u64,
    pub total_commands_processed: u64,
    pub total_sessions_failed:    u64,
    pub success_rate:             f64
}
