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

/// Command dispatcher that routes CLI commands to appropriate handlers
pub struct CommandDispatcher {
    event_bus: Arc<EventBus>
}

impl CommandDispatcher {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self { event_bus }
    }

    /// Dispatch a command based on CLI arguments
    pub async fn dispatch(&self, cli_args: &crate::shared::Cli, context: &CommandContext) -> Result<(), WorkflowError> {
        use crate::shared::commands::*;

        match &cli_args.command {
            Some(Commands::Init) => {
                let command = InitConfigCommand::new();
                self.event_bus.execute_command(command, context).await?;
            }
            Some(Commands::Lang { command: lang_cmd }) => {
                use crate::shared::LangCommands;
                match lang_cmd {
                    LangCommands::Set { language } => {
                        let command = SetLanguageCommand::new(language.clone());
                        self.event_bus.execute_command(command, context).await?;
                    }
                    LangCommands::List => {
                        // TODO: Implement ListLanguagesCommand
                        println!("Available languages: en, es");
                    }
                    LangCommands::Current => {
                        // TODO: Implement GetCurrentLanguageCommand
                        println!("Current language: en");
                    }
                }
            }
            Some(Commands::Resource { command: resource_cmd }) => {
                use crate::shared::ResourceCommands;
                match resource_cmd {
                    ResourceCommands::Set { url } => {
                        // TODO: Implement SetResourceUrlCommand
                        println!("Resource URL set to: {}", url);
                    }
                    ResourceCommands::Current => {
                        // TODO: Implement GetResourceUrlCommand
                        println!("Current resource URL: (not implemented)");
                    }
                }
            }
            Some(Commands::Sync { url, ssh_key }) => {
                let command = SyncWorkflowsCommand::new(url.clone(), ssh_key.clone());
                self.event_bus.execute_command(command, context).await?;
            }
            None => {
                // Handle workflow execution
                if cli_args.list {
                    let command = ListWorkflowsCommand::new();
                    self.event_bus.execute_command(command, context).await?;
                } else if let Some(file_path) = &cli_args.file {
                    let command = ExecuteWorkflowCommand::new(file_path.clone());
                    self.event_bus.execute_command(command, context).await?;
                } else {
                    // Interactive workflow selection
                    // TODO: Implement SelectWorkflowCommand
                    println!("Interactive workflow selection not yet implemented");
                }
            }
        }

        Ok(())
    }
}

// =============================================================================
// Built-in Event Handlers
// =============================================================================

/// Event handler that logs all events
pub struct LoggingEventHandler;

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
