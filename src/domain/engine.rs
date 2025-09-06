use ractor::ActorRef;

use crate::{
    actor::message::CommandProcessorMessage,
    domain::{command::WorkflowCommand, error::WorkflowError, workflow::WorkflowContext}
};

/// Engine execution context that provides commands access to workflow metadata
pub struct EngineContext {
    /// Workflow execution metadata (session_id, user, etc.)
    pub workflow_context: WorkflowContext,
    /// Actor reference for scheduling subsequent commands
    pub processor_ref:    ActorRef<CommandProcessorMessage>
}

impl EngineContext {
    pub fn new(workflow_context: WorkflowContext, processor_ref: ActorRef<CommandProcessorMessage>) -> Self {
        Self { workflow_context, processor_ref }
    }

    pub async fn schedule_command(&self, command: WorkflowCommand) -> Result<(), WorkflowError> {
        self.processor_ref
            .cast(CommandProcessorMessage::ScheduleCommand { command })
            .map_err(|e| WorkflowError::Generic(format!("Failed to schedule command: {:?}", e)))
    }
}
