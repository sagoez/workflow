//! Application Engine - The heart of the event-driven architecture
//!
//! The engine orchestrates the flow: Command → Load → Validate → Emit Events → Apply Side Effects

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use crate::shared::{Command, CommandContext, Event, EventBus, WorkflowError};

/// The central application engine that processes commands through the event-driven lifecycle
pub struct ApplicationEngine {
    event_bus:   Arc<dyn EventBus>,
    state_store: HashMap<String, Box<dyn std::any::Any + Send + Sync>>
}

impl ApplicationEngine {
    /// Create a new application engine
    pub fn new(event_bus: Arc<dyn EventBus>) -> Self {
        Self { event_bus, state_store: HashMap::new() }
    }

    /// Process a command through the full event-driven lifecycle
    ///
    /// 1. Load: Command loads any prerequisites
    /// 2. Validate: Command validates inputs and current state
    /// 3. Emit: Command emits domain events
    /// 4. Effect: Engine applies events to state and triggers side effects
    pub async fn execute_command<C: Command>(
        &mut self,
        command: C,
        context: &CommandContext
    ) -> Result<(), WorkflowError> {
        // Phase 1: Load prerequisites
        let loaded_command = command.load(context).await?;

        // Phase 2: Validate command and current state
        loaded_command.validate(context).await?;

        // Phase 3: Emit domain events
        let events = loaded_command.emit(context).await?;

        // Phase 4: Apply events and trigger side effects
        for event in events {
            // Apply event to state
            self.apply_event_to_state(&*event).await?;

            // Publish event for side effects
            self.event_bus.publish(event).await?;
        }

        Ok(())
    }

    /// Apply an event to the application state
    async fn apply_event_to_state(&mut self, event: &dyn Event) -> Result<(), WorkflowError> {
        let aggregate_id = event.aggregate_id().unwrap_or("global");
        let state_key = format!("{}:{}", event.state_type(), aggregate_id);

        // Get current state
        let current_state = self.state_store.get(&state_key);
        let current_state_ref = current_state.as_ref().map(|s| s.as_ref());

        // Apply event to produce new state
        if let Some(new_state) = event.apply(current_state_ref) {
            self.state_store.insert(state_key, new_state);
        } else {
            return Err(WorkflowError::Event(format!(
                "Failed to apply event {} to state {}",
                event.event_id(),
                event.state_type()
            )));
        }

        Ok(())
    }

    /// Get the current state for a given aggregate
    pub fn get_state<T: 'static>(&self, state_type: &str, aggregate_id: Option<&str>) -> Option<&T> {
        let aggregate_id = aggregate_id.unwrap_or("global");
        let state_key = format!("{}:{}", state_type, aggregate_id);

        self.state_store.get(&state_key)?.downcast_ref::<T>()
    }
}

/// Command dispatcher routes CLI commands to the appropriate command handlers
pub struct CommandDispatcher {
    engine: Arc<tokio::sync::Mutex<ApplicationEngine>>
}

impl CommandDispatcher {
    /// Create a new command dispatcher
    pub fn new(engine: Arc<tokio::sync::Mutex<ApplicationEngine>>) -> Self {
        Self { engine }
    }

    /// Dispatch a command through the engine
    pub async fn dispatch<C: Command>(&self, command: C, context: &CommandContext) -> Result<(), WorkflowError> {
        let mut engine = self.engine.lock().await;
        engine.execute_command(command, context).await
    }
}
