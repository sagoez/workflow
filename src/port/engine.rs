use async_trait::async_trait;

use crate::{
    domain::{
        command::WorkflowCommand, engine::EngineContext, error::WorkflowError, event::WorkflowEvent,
        state::WorkflowState
    },
    port::event::Event
};

/// Core engine trait for command execution
///
/// This trait defines the interface for different engine implementations,
/// allowing for versioned engines (EngineV1, EngineV2, etc.) with different
/// execution strategies and capabilities.
#[async_trait]
pub trait Engine: Send + Sync + 'static {
    /// Process a command through load → validate → emit phases
    ///
    /// Pure function: takes current state, returns events (no persistence)
    async fn process_command(
        &self,
        command: WorkflowCommand,
        context: &EngineContext,
        current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, WorkflowError>;

    /// Handle events and return the resulting state
    ///
    /// This applies events to state without side effects
    fn handle_events(
        &self,
        current_state: &WorkflowState,
        events: &[Box<dyn Event>]
    ) -> Result<WorkflowState, WorkflowError>;

    /// Execute the effect phase of a command
    ///
    /// This handles the side effects and external operations
    async fn effect(
        &self,
        loaded_data: &Box<dyn std::any::Any + Send + Sync>,
        command: WorkflowCommand,
        previous_state: &WorkflowState,
        current_state: &WorkflowState,
        context: &EngineContext
    ) -> Result<(), WorkflowError>;

    /// Get the engine version/name for identification
    fn engine_name(&self) -> &'static str;

    /// Get the engine version
    fn engine_version(&self) -> &'static str;
}
