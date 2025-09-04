//! Sync service - manages workflow synchronization from Git repositories
//!
//! This service handles all synchronization-related operations including:
//! - Cloning workflows from Git repositories
//! - Managing SSH authentication
//! - Tracking sync history and statistics
//! - Validating repository URLs

use std::{path::Path, sync::Arc};

use crate::{
    i18n,
    ports::{
        filesystem::FileSystem,
        git::{CloneOptions, GitClient},
        storage::{ConfigStore, EventData, EventStore},
        ui::{ProgressIndicator, Renderer}
    },
    shared::{CommandContext, WorkflowError, events::*}
};

/// Service for workflow synchronization operations
pub struct SyncService {
    config_store: Arc<dyn ConfigStore>,
    event_store:  Arc<dyn EventStore>,
    git_client:   Arc<dyn GitClient>,
    filesystem:   Arc<dyn FileSystem>,
    renderer:     Arc<dyn Renderer>,
    progress:     Arc<dyn ProgressIndicator>
}

impl SyncService {
    pub fn new(
        config_store: Arc<dyn ConfigStore>,
        event_store: Arc<dyn EventStore>,
        git_client: Arc<dyn GitClient>,
        filesystem: Arc<dyn FileSystem>,
        renderer: Arc<dyn Renderer>,
        progress: Arc<dyn ProgressIndicator>
    ) -> Self {
        Self { config_store, event_store, git_client, filesystem, renderer, progress }
    }

    /// Sync workflows from the configured Git repository
    pub async fn sync_workflows(&self, context: &CommandContext) -> Result<SyncResult, WorkflowError> {
        // Load repository URL and SSH key from configuration
        let config = self.config_store.load_config().await?;
        let repository_url =
            config.resource_url.ok_or_else(|| WorkflowError::Configuration(i18n::t("sync_no_url_configured")))?;

        // Use SSH key from config if available
        let ssh_key = config.ssh_key_path.as_deref();

        self.sync_from_url(&repository_url, ssh_key, context).await
    }

    /// Sync workflows from a specific Git repository URL
    pub async fn sync_from_url(
        &self,
        repository_url: &str,
        ssh_key: Option<&str>,
        context: &CommandContext
    ) -> Result<SyncResult, WorkflowError> {
        // Display sync start message
        self.renderer.display_message(&i18n::t_params("service_syncing_from", &[repository_url])).await?;

        // Start progress indicator
        let spinner = self.progress.start_spinner(&i18n::t("sync_cloning_repository")).await?;

        // Get workflows directory
        let config = self.config_store.load_config().await?;
        let workflows_dir = self.get_workflows_dir(&config).await?;

        // Ensure workflows directory exists
        self.filesystem.create_dir_all(Path::new(&workflows_dir)).await?;

        // Prepare clone options
        let clone_options = CloneOptions { ssh_key: ssh_key.map(|s| s.to_string()), ..Default::default() };

        // Clone the repository
        if let Err(e) =
            GitClient::clone(&*self.git_client, repository_url, Path::new(&workflows_dir), &clone_options).await
        {
            spinner.finish(Some(&i18n::t("sync_failed")));
            return Err(e);
        }

        // Get the current commit hash
        let commit_hash = match self.git_client.get_commit_info(Path::new(&workflows_dir), None).await {
            Ok(commit_info) => commit_info.short_id,
            Err(_) => "unknown".to_string() // Fallback if we can't get commit info
        };

        // Count workflow files
        let workflow_count = self.count_workflow_files(&workflows_dir).await?;

        // Finish progress indicator
        spinner.finish(Some(&i18n::t_params("service_synced_workflows", &[&workflow_count.to_string(), &commit_hash])));

        // Create and emit sync event
        let event = WorkflowsSyncedEvent::new(
            repository_url.to_string(),
            commit_hash.clone(),
            workflow_count,
            context.user.clone()
        );

        // Store the event
        let event_data = self.event_to_data(&event)?;
        self.event_store.save_event(&event_data).await?;

        // Display success message
        self.renderer.display_success(&i18n::t_params("sync_success", &[repository_url])).await?;

        Ok(SyncResult { repository_url: repository_url.to_string(), commit_hash, workflows_count: workflow_count })
    }

    /// Validate a Git repository URL
    pub async fn validate_repository_url(&self, url: &str) -> Result<bool, WorkflowError> {
        // Basic URL validation
        if !url.starts_with("http://") && !url.starts_with("https://") && !url.starts_with("git@") {
            return Err(WorkflowError::Validation(i18n::t_params("sync_invalid_url", &[url])));
        }

        // TODO: More sophisticated validation (check if repository exists, etc.)
        Ok(true)
    }

    /// Get sync history from events
    pub async fn get_sync_history(&self) -> Result<Vec<SyncHistoryEntry>, WorkflowError> {
        // Load sync events from event store (using "sync" as aggregate_id)
        let events = self.event_store.load_events("sync").await?;

        let mut history = Vec::new();
        for event_data in events {
            if let Ok(event) = serde_json::from_value::<WorkflowsSyncedEvent>(event_data.data) {
                history.push(SyncHistoryEntry {
                    timestamp:       event_data.timestamp,
                    repository_url:  event.repository_url,
                    commit_hash:     event.commit_hash,
                    workflows_count: event.workflows_count,
                    user:            event.user
                });
            }
        }

        // Sort by timestamp (most recent first)
        history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(history)
    }

    /// Display sync history
    pub async fn display_sync_history(&self) -> Result<(), WorkflowError> {
        let history = self.get_sync_history().await?;

        if history.is_empty() {
            self.renderer.display_message(&i18n::t("sync_no_history")).await?;
            return Ok(());
        }

        self.renderer.display_message(&i18n::t("sync_history_header")).await?;

        for entry in history.iter().take(10) {
            // Show last 10 syncs
            let formatted_time = entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC");
            self.renderer
                .display_message(&i18n::t_params(
                    "sync_history_entry",
                    &[
                        &formatted_time.to_string(),
                        &entry.repository_url,
                        &entry.commit_hash[..8], // Short commit hash
                        &entry.workflows_count.to_string(),
                        &entry.user
                    ]
                ))
                .await?;
        }

        Ok(())
    }

    // Private helper methods

    async fn get_workflows_dir(&self, _config: &crate::ports::storage::Config) -> Result<String, WorkflowError> {
        // TODO: Get workflows directory from config or use default
        Ok("./resource".to_string()) // Temporary default
    }

    async fn count_workflow_files(&self, workflows_dir: &str) -> Result<usize, WorkflowError> {
        let entries = self.filesystem.read_dir(Path::new(workflows_dir)).await?;
        let mut count = 0;

        for entry in entries {
            if let Some(extension) = entry.extension() {
                if extension == "yaml" || extension == "yml" {
                    // Skip config.yaml as it's not a workflow file
                    if let Some(file_name) = entry.file_name() {
                        if file_name != "config.yaml" {
                            count += 1;
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    fn event_to_data<E: serde::Serialize>(&self, event: &E) -> Result<EventData, WorkflowError> {
        let data = serde_json::to_value(event).map_err(|e| WorkflowError::Serialization(e.to_string()))?;

        Ok(EventData {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: "WorkflowsSynced".to_string(),
            aggregate_id: Some("sync".to_string()),
            timestamp: chrono::Utc::now(),
            data,
            metadata: None
        })
    }
}

/// Result of a sync operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub repository_url:  String,
    pub commit_hash:     String,
    pub workflows_count: usize
}

/// Entry in sync history
#[derive(Debug, Clone)]
pub struct SyncHistoryEntry {
    pub timestamp:       chrono::DateTime<chrono::Utc>,
    pub repository_url:  String,
    pub commit_hash:     String,
    pub workflows_count: usize,
    pub user:            String
}
