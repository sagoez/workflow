//! Application Engine - The heart of the event-driven architecture
//!
//! The engine orchestrates the flow: Command → Load → Validate → Emit Events → Apply Side Effects

use std::{collections::HashMap, sync::Arc};

use crate::shared::{Command, CommandContext, Event, EventBus, WorkflowError};

/// The central application engine that processes commands through the event-driven lifecycle
pub struct ApplicationEngine {
    event_bus:   Arc<EventBus>,
    state_store: HashMap<String, Box<dyn std::any::Any + Send + Sync>>
}

impl ApplicationEngine {
    /// Create a new application engine
    pub fn new(event_bus: Arc<EventBus>) -> Self {
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
        let mut loaded_command = command;
        loaded_command
            .load(context)
            .await
            .map_err(|_| WorkflowError::Execution("Failed to load command".to_string()))?;

        // Phase 2: Validate command and current state
        loaded_command.validate().map_err(|_| WorkflowError::Validation("Command validation failed".to_string()))?;

        // Phase 3: Emit domain events
        let events = loaded_command
            .emit(context)
            .await
            .map_err(|_| WorkflowError::Event("Failed to emit events".to_string()))?;

        // Phase 4: Apply events and trigger side effects
        for event in events {
            // Convert to Box<dyn Event>
            let event_box: Box<dyn Event> = event.clone_event();

            // Apply event to state
            self.apply_event_to_state(&*event_box).await?;

            // Store event in event store
            let event_json = event_box.to_json()?;
            let event_data = crate::ports::storage::EventData {
                event_id:     event_box.event_id().to_string(),
                event_type:   event_box.event_type().to_string(),
                aggregate_id: event_box.aggregate_id().map(|s| s.to_string()),
                timestamp:    event_box.timestamp(),
                data:         serde_json::from_str(&event_json)?,
                metadata:     None
            };
            self.event_bus.get_event_store().save_event(&event_data).await?;

            // Notify event handlers for side effects
            for handler in self.event_bus.get_handlers() {
                if handler.event_types().contains(&"*") || handler.event_types().contains(&event_box.event_type()) {
                    handler.handle_event(&*event_box, context).await?;
                }
            }
        }

        Ok(())
    }

    /// Apply an event to the application state
    async fn apply_event_to_state(&mut self, event: &dyn Event) -> Result<(), WorkflowError> {
        let aggregate_id = event.aggregate_id().unwrap_or("global");
        let state_key = format!("{}:{}", event.state_type(), aggregate_id);

        // Get current state
        let current_state = self.state_store.get(&state_key);
        let current_state_ref = current_state.as_ref().map(|s| s.as_ref() as &dyn std::any::Any);

        // Apply event to produce new state
        if let Some(new_state) = event.apply(current_state_ref) {
            // Convert Box<dyn Any> to Box<dyn Any + Send + Sync>
            let new_state: Box<dyn std::any::Any + Send + Sync> = unsafe { std::mem::transmute(new_state) };
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

    /// Dispatch CLI commands by routing them to appropriate command handlers
    pub async fn dispatch_cli(
        &self,
        cli_args: &crate::shared::Cli,
        context: &CommandContext
    ) -> Result<(), WorkflowError> {
        use crate::shared::commands::*;

        match &cli_args.command {
            Some(crate::shared::Commands::Init) => {
                let command = InitConfigCommand::new();
                self.dispatch(command, context).await?;
            }
            Some(crate::shared::Commands::Lang { command: lang_cmd }) => {
                use crate::shared::LangCommands;
                match lang_cmd {
                    LangCommands::Set { language } => {
                        let command = SetLanguageCommand::new(language.clone());
                        self.dispatch(command, context).await?;
                    }
                    LangCommands::Current => {
                        // TODO: Implement GetCurrentLanguageCommand
                        println!("Current language: en");
                    }
                    LangCommands::List => {
                        // TODO: Implement ListLanguagesCommand
                        println!("Listing languages");
                    }
                }
            }
            Some(crate::shared::Commands::Resource { command: resource_cmd }) => {
                use crate::shared::ResourceCommands;
                match resource_cmd {
                    ResourceCommands::Set { url } => {
                        // TODO: Implement SetResourceUrlCommand
                        println!("Resource URL set to: {}", url);
                    }
                }
            }
            Some(crate::shared::Commands::Sync { url, ssh_key }) => {
                let command = SyncWorkflowsCommand::new(url.clone(), ssh_key.clone());
                self.dispatch(command, context).await?;
            }
            None => {
                // Handle workflow execution
                if cli_args.list {
                    let command = ListWorkflowsCommand::new();
                    self.dispatch(command, context).await?;
                } else if let Some(file_path) = &cli_args.file {
                    let command = ExecuteWorkflowCommand::new(file_path.clone());
                    self.dispatch(command, context).await?;
                } else {
                    // Interactive workflow selection
                    let command = ListWorkflowsCommand::new();
                    self.dispatch(command, context).await?;
                }
            }
        }

        Ok(())
    }
}
