use std::path::Path;

use async_trait::async_trait;

use crate::{
    AppContext,
    adapter::storage::EventStoreType,
    domain::{
        command::PurgeStorageCommand, engine::EngineContext, error::WorkflowError, event::WorkflowEvent,
        state::WorkflowState
    },
    port::{command::Command, filesystem::FileSystem},
    t
};

/// Purge the database directory using the FileSystem trait.
/// Returns Ok(true) if purged, Ok(false) if nothing to purge.
pub fn purge_database(fs: &dyn FileSystem, db_path: &Path) -> Result<bool, WorkflowError> {
    if fs.exists(db_path) {
        fs.remove_dir_all(db_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[async_trait]
impl Command for PurgeStorageCommand {
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

    fn validate(&self, _loaded: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded: &Self::LoadedData,
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
        if app_context.config.storage_type != EventStoreType::RocksDb {
            println!("{}", t!("storage_purge_only_rocksdb"));
            return Ok(());
        }

        let db_path = app_context.config.database_path.clone();
        let fs = app_context.filesystem.clone();

        drop(app_context.event_store.clone());

        tokio::task::spawn_blocking(move || purge_database(&*fs, &db_path))
            .await
            .map_err(|e| WorkflowError::Generic(format!("Failed to purge storage: {}", e)))??;

        println!("{}", t!("storage_purge_success"));
        Ok(())
    }

    fn name(&self) -> &'static str {
        "purge_storage"
    }

    fn description(&self) -> &'static str {
        "Purge all data from storage"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::adapter::filesystem::mock::MockFileSystem;

    #[test]
    fn purge_removes_existing_directory() {
        let fs = MockFileSystem::new();
        let db_path = Path::new("/data/db");
        fs.create_dir_all(db_path).unwrap();
        fs.write(&db_path.join("data.sst"), "sst data").unwrap();

        let result = purge_database(&fs, db_path).unwrap();
        assert!(result);
        assert!(!fs.exists(db_path));
    }

    #[test]
    fn purge_nonexistent_returns_false() {
        let fs = MockFileSystem::new();
        let result = purge_database(&fs, Path::new("/data/db")).unwrap();
        assert!(!result);
    }
}
