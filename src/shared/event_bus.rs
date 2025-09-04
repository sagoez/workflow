//! Event bus and dispatcher for the event-driven architecture
//!
//! The event bus coordinates between commands, events, and side effects.
//! It ensures that all events are properly stored and that event handlers
//! are notified when events occur.

use std::sync::Arc;

use async_trait::async_trait;

use super::{Command, CommandContext, Event, WorkflowError};
use crate::ports::storage::{EventData, EventStore};

/// Event bus that coordinates command execution and event handling
pub struct EventBus {
    event_store: Arc<dyn EventStore>,
    handlers:    Vec<Box<dyn EventHandler>>
}

impl EventBus {
    pub fn new(event_store: Arc<dyn EventStore>) -> Self {
        Self { event_store, handlers: Vec::new() }
    }

    /// Register an event handler
    pub fn register_handler(&mut self, handler: Box<dyn EventHandler>) {
        self.handlers.push(handler);
    }

    /// Get the event store (for engine access)
    pub fn get_event_store(&self) -> Arc<dyn crate::ports::storage::EventStore> {
        self.event_store.clone()
    }

    /// Get the event handlers (for engine access)
    pub fn get_handlers(&self) -> &Vec<Box<dyn EventHandler>> {
        &self.handlers
    }

    /// Execute a command following the 4-phase lifecycle
    pub async fn execute_command<C>(&self, mut command: C, context: &CommandContext) -> Result<Vec<C::Event>, C::Error>
    where
        C: Command,
        C::Event: Event + Clone + serde::Serialize,
        C::Error: From<WorkflowError>
    {
        // Phase 1: Load
        command.load(context).await?;

        // Phase 2: Validate
        command.validate()?;

        // Phase 3: Emit
        let events = command.emit(context).await?;

        // Store events
        for event in &events {
            let event_data = self.event_to_data(event).map_err(C::Error::from)?;
            self.event_store.save_event(&event_data).await.map_err(C::Error::from)?;
        }

        // Notify handlers
        for event in &events {
            for handler in &self.handlers {
                if let Err(e) = handler.handle_event(event, context).await {
                    eprintln!("Event handler error: {}", e);
                    // Continue with other handlers even if one fails
                }
            }
        }

        // Phase 4: Effect
        command.effect(&events, context).await?;

        Ok(events)
    }

    /// Convert an event to EventData for storage
    fn event_to_data<E: Event + serde::Serialize>(&self, event: &E) -> Result<EventData, WorkflowError> {
        let data = serde_json::to_value(event).map_err(|e| WorkflowError::Serialization(e.to_string()))?;

        Ok(EventData {
            event_id: event.event_id().to_string(),
            event_type: event.event_type().to_string(),
            aggregate_id: event.aggregate_id().map(|s| s.to_string()),
            timestamp: event.timestamp(),
            data,
            metadata: None
        })
    }
}

/// Trait for handling events
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle an event
    async fn handle_event(&self, event: &dyn Event, context: &CommandContext) -> Result<(), WorkflowError>;

    /// Get the event types this handler is interested in
    fn event_types(&self) -> Vec<&'static str>;
}

// =============================================================================
// Built-in Event Handlers
// =============================================================================

/// Event handler that logs all events
pub struct LoggingEventHandler;

impl LoggingEventHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl EventHandler for LoggingEventHandler {
    async fn handle_event(&self, event: &dyn Event, _context: &CommandContext) -> Result<(), WorkflowError> {
        println!(
            "📝 Event: {} [{}] at {}",
            event.event_type(),
            event.event_id(),
            event.timestamp().format("%Y-%m-%d %H:%M:%S UTC")
        );
        Ok(())
    }

    fn event_types(&self) -> Vec<&'static str> {
        vec!["*"] // Handle all event types
    }
}

/// Event handler that updates workflow statistics
pub struct StatisticsEventHandler {
    // TODO: Add statistics tracking
}

impl StatisticsEventHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventHandler for StatisticsEventHandler {
    async fn handle_event(&self, event: &dyn Event, _context: &CommandContext) -> Result<(), WorkflowError> {
        match event.event_type() {
            "WorkflowStarted" => {
                // TODO: Increment workflow execution counter
            }
            "WorkflowCompleted" => {
                // TODO: Update success statistics
            }
            "WorkflowFailed" => {
                // TODO: Update failure statistics
            }
            _ => {}
        }
        Ok(())
    }

    fn event_types(&self) -> Vec<&'static str> {
        vec!["WorkflowStarted", "WorkflowCompleted", "WorkflowFailed"]
    }
}
