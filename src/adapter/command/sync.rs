use uuid::Uuid;

use crate::{
    AppContext,
    domain::{
        command::{RecordSyncResultCommand, SyncWorkflowsCommand},
        engine::EngineContext,
        error::WorkflowError,
        event::{SyncRequestedEvent, WorkflowEvent},
        state::WorkflowState
    },
    port::{command::Command, git::CloneOptions},
    t, t_params
};

#[derive(Debug, Clone)]
pub struct SyncWorkflowsData {
    pub remote_url: String,
    pub branch:     String,
    pub ssh_key:    Option<String>
}

/// Prepare sync data, applying default remote URL if none provided.
pub fn prepare_sync_data(remote_url: Option<&str>, branch: &str, ssh_key: Option<&str>) -> SyncWorkflowsData {
    let remote_url = remote_url.unwrap_or("https://github.com/sagoez/workflow-vault.git").to_string();

    SyncWorkflowsData { remote_url, branch: branch.to_string(), ssh_key: ssh_key.map(|s| s.to_string()) }
}

#[async_trait::async_trait]
impl Command for SyncWorkflowsCommand {
    type Error = WorkflowError;
    type LoadedData = SyncWorkflowsData;

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        Ok(prepare_sync_data(self.remote_url.as_deref(), &self.branch, self.ssh_key.as_deref()))
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
        let event = SyncRequestedEvent {
            event_id:   Uuid::new_v4().to_string(),
            timestamp:  chrono::Utc::now(),
            remote_url: loaded_data.remote_url.clone(),
            branch:     loaded_data.branch.clone(),
            ssh_key:    loaded_data.ssh_key.clone()
        };

        Ok(vec![WorkflowEvent::SyncRequested(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        context: &EngineContext,
        app_context: &AppContext
    ) -> Result<(), Self::Error> {
        match current_state {
            WorkflowState::SyncRequested(state) => {
                let workflows_dir = &app_context.config.workflows_dir;
                let clone_options =
                    CloneOptions { ssh_key: state.ssh_key.clone(), branch: Some(state.branch.clone()) };

                app_context.output.intro(&t_params!("cli_synced_workflows", &[&state.remote_url]));

                let spinner = app_context.output.spinner();
                spinner.start(&t_params!("git_cloning_from", &[&state.remote_url]));

                let commit_id =
                    app_context.git_client.clone_repository(&state.remote_url, workflows_dir, &clone_options).await?;

                spinner.stop(&t_params!("git_clone_success", &[&commit_id[..8]]));

                let record_sync_result_command = RecordSyncResultCommand { commit_id: commit_id.clone() };
                context.schedule_command(record_sync_result_command.into()).await?;
            }
            _ => {
                app_context.output.warning(&t!("error_sync_not_requested"));
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "sync-workflows"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_data_uses_default_url_when_none() {
        let data = prepare_sync_data(None, "main", None);
        assert_eq!(data.remote_url, "https://github.com/sagoez/workflow-vault.git");
        assert_eq!(data.branch, "main");
        assert!(data.ssh_key.is_none());
    }

    #[test]
    fn prepare_data_uses_explicit_url_and_ssh_key() {
        let data = prepare_sync_data(Some("git@github.com:user/repo.git"), "develop", Some("/path/to/key"));
        assert_eq!(data.remote_url, "git@github.com:user/repo.git");
        assert_eq!(data.branch, "develop");
        assert_eq!(data.ssh_key.as_deref(), Some("/path/to/key"));
    }
}
