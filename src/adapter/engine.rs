use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    AppContext,
    adapter::storage::EventStoreType,
    domain::{
        command::WorkflowCommand, engine::EngineContext, error::WorkflowError, event::WorkflowEvent,
        state::WorkflowState
    },
    port::{command::Command, engine::Engine, event::Event},
    t_params
};

pub struct EngineFactory;

impl EngineFactory {
    /// Create engine for actor-based system (no pipeline needed - actor IS the pipeline)
    pub fn init(_store_type: EventStoreType, app_context: AppContext) -> Arc<dyn Engine> {
        // Engine is now pure business logic - no persistence
        Arc::new(EngineV1::new(app_context))
    }
}

/// Version 1 of the workflow engine implementation
///
/// Pure business logic engine - no persistence (handled by PersistentActor)
pub struct EngineV1 {
    pub app_context: AppContext
}

impl EngineV1 {
    pub fn new(app_context: AppContext) -> Self {
        Self { app_context }
    }
}

#[async_trait]
impl Engine for EngineV1 {
    async fn process_command(
        &self,
        command: WorkflowCommand,
        context: &EngineContext,
        current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, WorkflowError> {
        let loaded_data = command
            .load(context, &self.app_context, &current_state)
            .await
            .map_err(|e| WorkflowError::Execution(t_params!("load_phase_failed", &[&e.to_string()])))?;

        command
            .validate(&loaded_data)
            .map_err(|e| WorkflowError::Validation(t_params!("validation_phase_failed", &[&e.to_string()])))?;

        let events = command
            .emit(&loaded_data, context, &self.app_context, &current_state)
            .await
            .map_err(|e| WorkflowError::Event(t_params!("emit_phase_failed", &[&e.to_string()])))?;

        Ok(events)
    }

    fn handle_events(
        &self,
        current_state: &WorkflowState,
        events: &[Box<dyn Event>]
    ) -> Result<WorkflowState, WorkflowError> {
        let mut state = current_state.clone();

        for event in events {
            state = event
                .apply(Some(&state))
                .ok_or_else(|| WorkflowError::Event(t_params!("failed_to_apply_event", &[&format!("{:?}", event)])))?;
        }

        Ok(state)
    }

    async fn effect(
        &self,
        command: WorkflowCommand,
        previous_state: &WorkflowState,
        current_state: &WorkflowState,
        context: &EngineContext
    ) -> Result<(), WorkflowError> {
        command
            .effect(previous_state, current_state, context, &self.app_context)
            .await
            .map_err(|e| WorkflowError::Execution(t_params!("effect_phase_failed", &[&e.to_string()])))?;
        Ok(())
    }

    // Persistence methods removed - handled by PersistentActor

    fn engine_name(&self) -> &'static str {
        "EngineV1"
    }

    fn engine_version(&self) -> &'static str {
        "1.0.0"
    }
}
