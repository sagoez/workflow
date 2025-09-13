//! CommandProcessor Actor - Clean and Simple
//!
//! Handles workflow command processing for a single session using:
//! - Engine: Pure business logic
//! - Journal: Pluggable event persistence (session_id as persistence_id)

use std::sync::Arc;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tracing::{Level, event};

use crate::{
    AppContext,
    actor::message::CommandProcessorMessage,
    domain::{
        command::WorkflowCommand, constant::command_processor, engine::EngineContext, error::WorkflowError,
        state::WorkflowState, workflow::WorkflowContext
    },
    port::{command::Command, engine::Engine, journal::Journal},
    t_params
};

/// CommandProcessor Actor State - clean and focused
pub struct CommandProcessorState {
    /// Session ID (this IS the persistence_id in Journal!)
    pub session_id:       String,
    /// Engine for pure business logic
    pub engine:           Arc<dyn Engine>,
    /// Journal for event persistence (pluggable!)
    pub journal:          Arc<dyn Journal>,
    /// Workflow context
    pub workflow_context: WorkflowContext,
    /// Application context
    pub app_context:      Arc<AppContext>,
    /// Current workflow state (recovered from journal)
    pub current_state:    WorkflowState
}

/// CommandProcessor Actor - handles commands for a single workflow session
pub struct CommandProcessor;

#[async_trait::async_trait]
impl Actor for CommandProcessor {
    type Arguments = (String, Arc<dyn Engine>, Arc<dyn Journal>, Arc<AppContext>);
    type Msg = CommandProcessorMessage;
    type State = CommandProcessorState;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        (session_id, engine, journal, app_context): Self::Arguments
    ) -> Result<Self::State, ActorProcessingErr> {
        event!(Level::DEBUG, event = command_processor::PROCESSOR_STARTED, session_id = %session_id);

        let workflow_context = WorkflowContext::with_session_id(&session_id);

        // Recover state from journal (session_id as persistence_id)
        let current_state = match Self::recover_state(&session_id, &journal).await {
            Ok(state) => {
                event!(Level::DEBUG, event = command_processor::PROCESSOR_STARTED,
                       session_id = %session_id, message = "state_recovered");
                state
            }
            Err(e) => {
                event!(Level::WARN, event = command_processor::PROCESSOR_STARTED,
                       session_id = %session_id, error = %e, message = "recovery_failed_starting_fresh");
                WorkflowState::default()
            }
        };

        Ok(CommandProcessorState { session_id, engine, journal, workflow_context, app_context, current_state })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State
    ) -> Result<(), ActorProcessingErr> {
        match message {
            CommandProcessorMessage::ProcessCommand { command, reply } => {
                let result = self.handle_process_command(myself, command, state).await;
                let _ = reply.send(result);
                Ok(())
            }
            CommandProcessorMessage::ScheduleCommand { command } => {
                // Just process it directly - no need for complex scheduling
                self.handle_process_command(myself, command, state).await
            }
            CommandProcessorMessage::Complete => self.handle_complete(state).await
        }
    }
}

impl CommandProcessor {
    /// Handle command processing - the core workflow
    async fn handle_process_command(
        &self,
        myself: ActorRef<CommandProcessorMessage>,
        command: WorkflowCommand,
        state: &mut CommandProcessorState
    ) -> Result<(), ActorProcessingErr> {
        event!(Level::DEBUG, event = command_processor::COMMAND_RECEIVED,
               command = %command.name(), session_id = %state.session_id);

        let engine_context = EngineContext::new(state.workflow_context.clone(), myself.clone());

        match self.process_command(&command, &engine_context, state).await {
            Ok(()) => {
                event!(Level::DEBUG, event = command_processor::COMMAND_PROCESSED,
                       session_id = %state.session_id, command = %command.name());
                Ok(())
            }
            Err(e) => {
                event!(Level::ERROR, event = command_processor::COMMAND_FAILED,
                       session_id = %state.session_id, command = %command.name(), error = %e);
                Err(ActorProcessingErr::from(t_params!("error_command_processing_failed", &[&e.to_string()])))
            }
        }
    }

    /// Core command processing logic - clean and simple!
    async fn process_command(
        &self,
        command: &WorkflowCommand,
        context: &EngineContext,
        state: &mut CommandProcessorState
    ) -> Result<(), WorkflowError> {
        // 1. Engine processes command (pure business logic)
        let events = state.engine.process_command(command.clone(), context, &state.current_state).await?;

        // 2. Persist events to journal (session_id as persistence_id)
        if !events.is_empty() {
            state.journal.persist_events(&state.session_id, &events).await?;
        }

        // 3. Apply events to get new state
        let boxed_events: Vec<Box<dyn crate::port::event::Event>> =
            events.iter().map(|e| Box::new(e.clone()) as Box<dyn crate::port::event::Event>).collect();
        state.current_state = state.engine.handle_events(&state.current_state, &boxed_events)?;

        // 4. Execute effects (side effects)
        state.engine.effect(command.clone(), &state.current_state, &state.current_state, context).await?;

        Ok(())
    }

    /// Recover state from journal on startup
    async fn recover_state(session_id: &str, journal: &Arc<dyn Journal>) -> Result<WorkflowState, WorkflowError> {
        let events = journal.replay_events(session_id, 0).await?;

        if events.is_empty() {
            return Ok(WorkflowState::default());
        }

        // TODO: Apply events to rebuild state
        // For now, just return default state
        Ok(WorkflowState::default())
    }

    /// Handle session completion
    async fn handle_complete(&self, state: &mut CommandProcessorState) -> Result<(), ActorProcessingErr> {
        event!(Level::DEBUG, event = command_processor::SESSION_COMPLETED,
               session_id = %state.session_id);
        Ok(())
    }
}
