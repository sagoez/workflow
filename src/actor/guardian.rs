//! Guardian Actor - Root Supervisor
//!
//! The Guardian is the root of the actor hierarchy and manages system-wide concerns:
//! - Spawns and supervises WorkflowManager
//! - Handles system initialization and shutdown
//! - Provides health checks and system monitoring

use std::{sync::Arc, time::SystemTime};

use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, SpawnErr, rpc::CallResult};
use tracing::{Level, event};

use crate::{
    AppContext,
    actor::{
        manager::WorkflowManager,
        message::{GuardianMessage, SystemHealth, WorkflowManagerMessage}
    },
    domain::{command::WorkflowCommand, constant::guardian, error::WorkflowError, workflow::WorkflowContext},
    t, t_params
};

/// Guardian Actor State - tracks child actors and system metrics
pub struct GuardianState {
    /// WorkflowManager actor reference
    workflow_manager: Option<ActorRef<WorkflowManagerMessage>>,
    /// System startup time for uptime calculation
    startup_time:     SystemTime,
    /// System initialization flag
    is_initialized:   bool
}

/// Guardian Actor - Root supervisor of the actor system
pub struct Guardian;

#[async_trait::async_trait]
impl Actor for Guardian {
    type Arguments = ();
    type Msg = GuardianMessage;
    type State = GuardianState;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments
    ) -> Result<Self::State, ActorProcessingErr> {
        event!(Level::DEBUG, event = guardian::GUARDIAN_STARTED);

        Ok(GuardianState { workflow_manager: None, startup_time: SystemTime::now(), is_initialized: false })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State
    ) -> Result<(), ActorProcessingErr> {
        match message {
            GuardianMessage::Initialize => self.handle_initialize(myself, state).await,
            GuardianMessage::Shutdown => self.handle_shutdown(state).await,
            GuardianMessage::HealthCheck { reply } => self.handle_health_check(reply, state).await,
            GuardianMessage::SubmitCommand { command, context, reply } => {
                self.handle_submit_command(command, *context, reply, state).await
            }
        }
    }
}

impl Guardian {
    /// Spawn the complete actor system
    pub async fn spawn_system() -> Result<ActorRef<GuardianMessage>, SpawnErr> {
        let (guardian_ref, _handle) = Actor::spawn(Some("guardian".to_string()), Guardian, ()).await?;

        // Initialize the system
        if let Err(e) = guardian_ref.cast(GuardianMessage::Initialize) {
            event!(Level::ERROR, event = guardian::GUARDIAN_STARTED, error = ?e);
        }

        Ok(guardian_ref)
    }

    /// Initialize child actors
    async fn handle_initialize(
        &self,
        _myself: ActorRef<GuardianMessage>,
        state: &mut GuardianState
    ) -> Result<(), ActorProcessingErr> {
        event!(Level::DEBUG, event = guardian::CHILDREN_SPAWNING);

        // Create AppContext
        let app_context = Arc::new(AppContext::init().map_err(|e| {
            ActorProcessingErr::from(t_params!("error_failed_to_create_app_context", &[&e.to_string()]))
        })?);

        // Spawn WorkflowManager
        match Actor::spawn(Some("workflow_manager".to_string()), WorkflowManager, app_context).await {
            Ok((workflow_manager_ref, _handle)) => {
                state.workflow_manager = Some(workflow_manager_ref);
                state.is_initialized = true;
                event!(Level::DEBUG, event = guardian::CHILDREN_SPAWNED, actor = "workflow_manager");
            }
            Err(e) => {
                event!(Level::ERROR, event = guardian::CHILDREN_SPAWN_FAILED, actor = "workflow_manager", error = %e);
                return Err(ActorProcessingErr::from(t_params!(
                    "error_failed_to_spawn_workflow_manager",
                    &[&e.to_string()]
                )));
            }
        }

        event!(Level::INFO, event = guardian::SYSTEM_INITIALIZED);
        Ok(())
    }

    /// Shutdown child actors gracefully
    async fn handle_shutdown(&self, state: &mut GuardianState) -> Result<(), ActorProcessingErr> {
        event!(Level::DEBUG, event = guardian::SYSTEM_SHUTDOWN_STARTED);

        if let Some(workflow_manager) = &state.workflow_manager {
            workflow_manager.stop(None);
            event!(Level::DEBUG, event = guardian::SYSTEM_SHUTDOWN_STARTED, actor = "workflow_manager_stopped");
        }

        state.is_initialized = false;
        event!(Level::INFO, event = guardian::SYSTEM_SHUTDOWN_COMPLETED);
        Ok(())
    }

    /// Handle health check requests
    async fn handle_health_check(
        &self,
        reply: ractor::RpcReplyPort<SystemHealth>,
        state: &GuardianState
    ) -> Result<(), ActorProcessingErr> {
        let uptime_seconds = state.startup_time.elapsed().unwrap_or_default().as_secs();

        let active_sessions = if let Some(workflow_manager) = &state.workflow_manager {
            match ractor::rpc::call(workflow_manager, |reply| WorkflowManagerMessage::GetActiveSessions { reply }, None)
                .await
            {
                Ok(CallResult::Success(count)) => count,
                _ => 0
            }
        } else {
            0
        };

        let health = SystemHealth {
            active_sessions,
            total_commands_processed: 0, // TODO: Get from WorkflowManager
            uptime_seconds
        };

        event!(Level::DEBUG, event = guardian::HEALTH_CHECK_COMPLETED,
               active_sessions = %active_sessions, uptime_seconds = %uptime_seconds);

        if let Err(e) = reply.send(health) {
            event!(Level::ERROR, event = guardian::HEALTH_CHECK_COMPLETED, error = %e);
        }

        Ok(())
    }

    /// Handle command submission
    async fn handle_submit_command(
        &self,
        command: WorkflowCommand,
        context: WorkflowContext,
        reply: RpcReplyPort<Result<(), WorkflowError>>,
        state: &GuardianState
    ) -> Result<(), ActorProcessingErr> {
        if let Some(workflow_manager) = &state.workflow_manager {
            match ractor::rpc::call(
                workflow_manager,
                |wm_reply| WorkflowManagerMessage::SubmitCommand {
                    command,
                    context: Box::new(context),
                    reply: wm_reply
                },
                None
            )
            .await
            {
                Ok(CallResult::Success(result)) => {
                    if let Err(e) = reply.send(result) {
                        event!(Level::ERROR, event = guardian::COMMAND_SUBMITTED, error = %e);
                    }
                }
                Ok(_) => {
                    if let Err(e) = reply.send(Err(WorkflowError::Generic(t!("error_workflow_manager_call_failed")))) {
                        event!(Level::ERROR, event = guardian::COMMAND_SUBMITTED, error = %e);
                    }
                }
                Err(e) => {
                    if let Err(send_err) = reply.send(Err(WorkflowError::Generic(t_params!(
                        "error_failed_to_submit_command",
                        &[&format!("{:?}", e.to_string())]
                    )))) {
                        event!(Level::ERROR, event = guardian::COMMAND_SUBMITTED, error = %send_err);
                    }
                }
            }
        } else if let Err(e) = reply.send(Err(WorkflowError::Generic(t!("error_actor_system_not_initialized")))) {
            event!(Level::ERROR, event = guardian::COMMAND_SUBMITTED, error = %e);
        }

        Ok(())
    }
}
