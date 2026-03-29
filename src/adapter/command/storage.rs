use async_trait::async_trait;

use crate::{
    AppContext,
    adapter::storage::EventStoreType,
    domain::{
        command::{GetCurrentStorageCommand, SetStorageCommand},
        engine::EngineContext,
        error::WorkflowError,
        event::WorkflowEvent,
        state::WorkflowState
    },
    port::command::Command,
    t_params
};

#[async_trait]
impl Command for SetStorageCommand {
    type Error = WorkflowError;
    type LoadedData = ();

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        Ok(())
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        app_context.config.set_current_storage(self.backend)?;
        Ok(vec![])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        _current_state: &WorkflowState,
        _context: &EngineContext,
        app_context: &AppContext
    ) -> Result<(), Self::Error> {
        app_context.output.success(&t_params!("storage_set_success", &[self.backend.as_str()]));
        Ok(())
    }

    fn name(&self) -> &'static str {
        "set-storage"
    }

    fn description(&self) -> &'static str {
        "Sets the storage backend"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

#[async_trait]
impl Command for GetCurrentStorageCommand {
    type Error = WorkflowError;
    type LoadedData = EventStoreType;

    async fn load(
        &self,
        _context: &EngineContext,
        app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        app_context.config.get_current_storage()
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        Ok(vec![])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        _current_state: &WorkflowState,
        _context: &EngineContext,
        app_context: &AppContext
    ) -> Result<(), Self::Error> {
        let current = app_context.config.get_current_storage()?;
        app_context.output.info(&t_params!("storage_current", &[current.as_str()]));
        Ok(())
    }

    fn name(&self) -> &'static str {
        "get-current-storage"
    }

    fn description(&self) -> &'static str {
        "Gets the current storage backend"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::storage::EventStoreType;

    #[test]
    fn set_storage_command_metadata() {
        let cmd = SetStorageCommand { backend: EventStoreType::InMemory };
        assert_eq!(cmd.name(), "set-storage");
        assert!(cmd.is_mutating());
        assert!(!cmd.is_interactive());
    }

    #[test]
    fn get_current_storage_command_metadata() {
        let cmd = GetCurrentStorageCommand;
        assert_eq!(cmd.name(), "get-current-storage");
        assert!(!cmd.is_mutating());
        assert!(!cmd.is_interactive());
    }
}
