use async_trait::async_trait;
use clipboard::ClipboardProvider;

use crate::{
    AppContext,
    domain::{
        command::WorkflowCommand, engine::EngineContext, error::WorkflowError, event::WorkflowEvent,
        state::WorkflowState
    },
    port::command::Command,
    t, t_params
};

pub mod aggregate;
pub mod complete;
pub mod discover;
pub mod language;
pub mod list;
pub mod purge;
pub mod resolve;
pub mod select;
pub mod start;
pub mod storage;
pub mod sync;
pub mod sync_record;

/// Macro to implement Command trait for WorkflowCommand enum
/// Similar to the impl_event macro for WorkflowEvent
macro_rules! impl_command {
    ($enum_name:ident { $($variant:ident($field:ident)),* $(,)? }) => {
        #[async_trait]
        impl Command for $enum_name {
            type Error = WorkflowError;
            type LoadedData = Box<dyn std::any::Any + Send + Sync>; // Generic type since each command has
                // different data

            async fn load(
                &self,
                context: &EngineContext,
                app_context: &AppContext,
                current_state: &WorkflowState
            ) -> Result<Self::LoadedData, Self::Error> {
                match self {
                    $(
                        $enum_name::$variant($field) => {
                            let data = $field.load(context, app_context, current_state).await?;
                            Ok(Box::new(data) as Box<dyn std::any::Any + Send + Sync>)
                        }
                    )*
                }
            }

            fn validate(&self, loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
                match self {
                    $(
                        $enum_name::$variant($field) => {
                            let data = loaded_data.downcast_ref().ok_or_else(||
                                WorkflowError::Other(t!("error_failed_to_downcast_loaded_data").to_string()))?;
                            $field.validate(data)
                        }
                    )*
                }
            }

            async fn emit(
                &self,
                loaded_data: &Self::LoadedData,
                context: &EngineContext,
                app_context: &AppContext,
                current_state: &WorkflowState
            ) -> Result<Vec<WorkflowEvent>, Self::Error> {
                match self {
                    $(
                        $enum_name::$variant($field) => {
                            let data = loaded_data.downcast_ref().ok_or_else(||
                                WorkflowError::Other(t!("error_failed_to_downcast_loaded_data").to_string()))?;
                            $field.emit(data, context, app_context, current_state).await
                        }
                    )*
                }
            }

            async fn effect(
                &self,
                loaded_data: &Self::LoadedData,
                previous_state: &WorkflowState,
                current_state: &WorkflowState,
                context: &EngineContext,
                app_context: &AppContext
            ) -> Result<(), Self::Error> {
                match self {
                    $(
                        $enum_name::$variant($field) => {
                            let loaded_data = loaded_data.downcast_ref().ok_or_else(||
                                WorkflowError::Other(t!("error_failed_to_downcast_loaded_data").to_string()))?;
                            $field.effect(loaded_data, previous_state, current_state, context, app_context).await
                        }
                    )*
                }
            }

            fn name(&self) -> &'static str {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.name(),
                    )*
                }
            }

            fn description(&self) -> &'static str {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.description(),
                    )*
                }
            }

            fn is_interactive(&self) -> bool {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.is_interactive(),
                    )*
                }
            }

            fn is_mutating(&self) -> bool {
                match self {
                    $(
                        $enum_name::$variant($field) => $field.is_mutating(),
                    )*
                }
            }
        }
    };
}

impl_command!(WorkflowCommand {
    DiscoverWorkflows(cmd),
    ListWorkflows(cmd),
    InteractivelySelectWorkflow(cmd),
    StartWorkflow(cmd),
    CompleteWorkflow(cmd),
    ResolveArguments(cmd),
    SyncWorkflows(cmd),
    RecordSyncResult(cmd),
    SetLanguage(cmd),
    GetCurrentLanguage(cmd),
    ListLanguages(cmd),
    SetStorage(cmd),
    GetCurrentStorage(cmd),
    ListAggregates(cmd),
    ReplayAggregate(cmd),
    DeleteAggregate(cmd),
    PurgeStorage(cmd)
});

/// Helper function to copy text to clipboard
fn copy_to_clipboard(text: &str) -> Result<(), WorkflowError> {
    let mut ctx = clipboard::ClipboardContext::new()
        .map_err(|e| WorkflowError::Other(t_params!("error_failed_to_create_clipboard_context", &[&e.to_string()])))?;
    ctx.set_contents(text.to_owned())
        .map_err(|e| WorkflowError::Other(t_params!("error_failed_to_set_clipboard_contents", &[&e.to_string()])))?;
    Ok(())
}
