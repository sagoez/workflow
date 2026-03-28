use std::path::Path;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    AppContext,
    domain::{
        command::RecordSyncResultCommand,
        engine::EngineContext,
        error::{ValidationError, WorkflowError},
        event::{WorkflowEvent, WorkflowsSyncedEvent},
        state::WorkflowState
    },
    port::{command::Command, filesystem::FileSystem},
    t, t_params
};

#[derive(Debug, Clone)]
pub struct RecordSyncResultData {
    pub remote_url:   String,
    pub branch:       String,
    pub commit_id:    String,
    pub synced_count: u32
}

/// Count YAML/YML workflow files in a directory using the FileSystem trait.
pub fn count_workflow_files(fs: &dyn FileSystem, dir: &Path) -> Result<u32, WorkflowError> {
    if !fs.exists(dir) {
        return Ok(0);
    }

    let entries = fs.read_dir_entries(dir)?;
    let count =
        entries.iter().filter(|p| p.extension().map(|ext| ext == "yaml" || ext == "yml").unwrap_or(false)).count()
            as u32;

    Ok(count)
}

#[async_trait]
impl Command for RecordSyncResultCommand {
    type Error = WorkflowError;
    type LoadedData = RecordSyncResultData;

    async fn load(
        &self,
        _context: &EngineContext,
        app_context: &AppContext,
        current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        let sync_state = match current_state {
            WorkflowState::SyncRequested(sync_state) => sync_state,
            _ => {
                return Err(ValidationError::InvalidState(
                    "Cannot record sync results: no sync was requested".to_string()
                )
                .into());
            }
        };

        let workflows_dir = &app_context.config.workflows_dir;
        let commit_id = self.commit_id.clone();
        let synced_count = count_workflow_files(&*app_context.filesystem, workflows_dir)?;

        Ok(RecordSyncResultData {
            remote_url: sync_state.remote_url.clone(),
            branch: sync_state.branch.clone(),
            commit_id,
            synced_count
        })
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        let event = WorkflowsSyncedEvent {
            event_id:     Uuid::new_v4().to_string(),
            timestamp:    chrono::Utc::now(),
            remote_url:   loaded_data.remote_url.clone(),
            branch:       loaded_data.branch.clone(),
            commit_id:    loaded_data.commit_id.clone(),
            synced_count: loaded_data.synced_count
        };

        Ok(vec![WorkflowEvent::WorkflowsSynced(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        match current_state {
            WorkflowState::WorkflowsSynced(state) => {
                println!("{}", t_params!("cli_synced_workflows", &[&state.remote_url]));
                println!("{}", t_params!("cli_synced_branch", &[&state.branch]));
                println!("{}", t_params!("cli_synced_commit", &[&state.commit_id]));
                println!("{}", t_params!("cli_synced_count", &[&state.synced_count.to_string()]));
            }
            _ => {
                println!("{}", t!("error_no_workflows_synced"));
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "record-sync-result"
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::adapter::filesystem::mock::MockFileSystem;

    #[test]
    fn counts_yaml_files() {
        let fs = MockFileSystem::new();
        let dir = Path::new("/workflows");
        fs.create_dir_all(dir).unwrap();
        fs.write(&dir.join("a.yaml"), "content").unwrap();
        fs.write(&dir.join("b.yml"), "content").unwrap();
        fs.write(&dir.join("c.txt"), "content").unwrap();

        let count = count_workflow_files(&fs, dir).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn nonexistent_dir_returns_zero() {
        let fs = MockFileSystem::new();
        let count = count_workflow_files(&fs, Path::new("/nope")).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn empty_dir_returns_zero() {
        let fs = MockFileSystem::new();
        let dir = Path::new("/workflows");
        fs.create_dir_all(dir).unwrap();

        let count = count_workflow_files(&fs, dir).unwrap();
        assert_eq!(count, 0);
    }
}
