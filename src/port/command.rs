//! Base command trait that all commands must implement
//!
//! Every command follows the four-phase lifecycle:
//! 1. Load - Gather prerequisites and dependencies
//! 2. Validate - Ensure command can be executed safely
//! 3. Emit - Generate events representing what will happen
//! 4. Effect - Execute side effects and external operations

use std::fmt::Debug;

use async_trait::async_trait;

use crate::{
    AppContext,
    domain::{engine::EngineContext, event::WorkflowEvent, state::WorkflowState}
};

/// Base trait that all commands must implement
#[async_trait]
pub trait Command: Debug + Send + Sync {
    /// The type of data loaded by this command
    type LoadedData: Send + Sync;

    /// The type of errors this command can produce
    type Error: std::error::Error + Send + Sync + 'static;

    /// Phase 1: Load prerequisites and dependencies
    ///
    /// This phase should:
    /// - Load required data from files, databases, APIs
    /// - Resolve user inputs and arguments
    /// - Gather any dependencies needed for execution
    /// - Return the loaded data for validation and event emission
    async fn load(
        &self,
        context: &EngineContext,
        app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error>;

    /// Phase 2: Validate that the command can be executed
    ///
    /// This phase should:
    /// - Validate all loaded data is correct
    /// - Check permissions and access rights
    /// - Verify preconditions are met
    /// - Ensure no conflicts with current state
    fn validate(&self, loaded_data: &Self::LoadedData) -> Result<(), Self::Error>;

    /// Phase 3: Emit events representing what will happen
    ///
    /// This phase should:
    /// - Generate events for all state changes that will occur
    /// - Create audit trail events
    /// - Emit events for external observers
    /// - Return events in chronological order
    async fn emit(
        &self,
        loaded_data: &Self::LoadedData,
        context: &EngineContext,
        app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error>;

    /// Phase 4: Execute side effects and external operations
    ///
    /// This phase should:
    /// - Perform I/O operations
    /// - Make external API calls
    /// - Update files and databases
    /// - Display output to user
    /// - Execute system commands
    async fn effect(
        &self,
        previous_state: &WorkflowState,
        current_state: &WorkflowState,
        context: &EngineContext,
        app_context: &AppContext
    ) -> Result<(), Self::Error>;

    /// Get a human-readable name for this command (for logging/debugging)
    fn name(&self) -> &'static str;

    /// Get a description of what this command does
    fn description(&self) -> &'static str {
        "No description provided"
    }

    /// Check if this command requires user interaction
    fn is_interactive(&self) -> bool {
        false
    }

    /// Check if this command modifies system state
    fn is_mutating(&self) -> bool {
        true
    }
}

/// Trait for commands that can be undone
#[async_trait]
pub trait UndoableCommand: Command {
    /// Generate events to undo the effects of this command
    async fn undo_events(
        &self,
        context: &EngineContext,
        app_context: &AppContext
    ) -> Result<Vec<WorkflowEvent>, Self::Error>;

    /// Execute side effects to undo this command
    async fn undo_effect(
        &self,
        events: &[WorkflowEvent],
        context: &EngineContext,
        app_context: &AppContext
    ) -> Result<(), Self::Error>;
}

/// Trait for commands that can provide progress updates
#[async_trait]
pub trait ProgressCommand: Command {
    /// Get the total number of steps for progress tracking
    fn total_steps(&self) -> usize;

    /// Get the current step number
    fn current_step(&self) -> usize;

    /// Get a description of the current step
    fn current_step_description(&self) -> String;
}
