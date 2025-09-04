//! Dependency injection container for the workflow system
//!
//! This module provides a container that wires together all the services,
//! adapters, and infrastructure components needed for the application.

use std::sync::Arc;

use crate::{
    adapters::{
        filesystem::LocalFileSystem,
        git::Git2Client,
        network::ReqwestClient,
        storage::{FileConfigStore, RocksDbEventStore, RocksDbHistoryStore},
        ui::{TerminalProgressIndicator, TerminalPrompter, TerminalRenderer}
    },
    ports::{
        filesystem::FileSystem,
        git::GitClient,
        network::HttpClient,
        storage::{ConfigStore, EventStore, HistoryStore},
        ui::{ProgressIndicator, Prompter, Renderer}
    },
    services::{ConfigService, SyncService, WorkflowService},
    shared::{CommandDispatcher, EventBus, LoggingEventHandler, StatisticsEventHandler}
};

/// Application container that holds all dependencies
pub struct Container {
    pub event_bus:          Arc<EventBus>,
    pub command_dispatcher: Arc<CommandDispatcher>,
    pub workflow_service:   Arc<WorkflowService>,
    pub config_service:     Arc<ConfigService>,
    pub sync_service:       Arc<SyncService>
}

impl Container {
    /// Create a new container with all dependencies wired up
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Infrastructure adapters
        let filesystem = Arc::new(LocalFileSystem::new()) as Arc<dyn FileSystem>;
        let git_client = Arc::new(Git2Client::new()) as Arc<dyn GitClient>;
        let http_client = Arc::new(ReqwestClient::new()) as Arc<dyn HttpClient>;

        // UI adapters
        let prompter = Arc::new(TerminalPrompter::new()) as Arc<dyn Prompter>;
        let renderer = Arc::new(TerminalRenderer::new()) as Arc<dyn Renderer>;
        let progress = Arc::new(TerminalProgressIndicator::new()) as Arc<dyn ProgressIndicator>;

        // Storage adapters
        let storage_config = crate::StorageConfig::default();
        let event_store = Arc::new(RocksDbEventStore::new(&storage_config).await?) as Arc<dyn EventStore>;
        let history_store = Arc::new(RocksDbHistoryStore::new(&storage_config).await?) as Arc<dyn HistoryStore>;

        // Config store (file-based for now, could be RocksDB later)
        let config_store = Arc::new(FileConfigStore::new()) as Arc<dyn ConfigStore>;

        // Event bus and handlers
        let mut event_bus = EventBus::new(event_store.clone());
        event_bus.register_handler(Box::new(LoggingEventHandler));
        event_bus.register_handler(Box::new(StatisticsEventHandler::new()));
        let event_bus = Arc::new(event_bus);

        // Command dispatcher
        let command_dispatcher = Arc::new(CommandDispatcher::new(event_bus.clone()));

        // Services
        let workflow_service = Arc::new(WorkflowService::new(
            filesystem.clone(),
            event_store.clone(),
            history_store.clone(),
            config_store.clone(),
            prompter.clone(),
            renderer.clone(),
            progress.clone()
        ));

        let config_service = Arc::new(ConfigService::new(
            config_store.clone(),
            event_store.clone(),
            filesystem.clone(),
            renderer.clone(),
            progress.clone()
        ));

        let sync_service = Arc::new(SyncService::new(
            config_store.clone(),
            event_store.clone(),
            git_client.clone(),
            filesystem.clone(),
            renderer.clone(),
            progress.clone()
        ));

        Ok(Self { event_bus, command_dispatcher, workflow_service, config_service, sync_service })
    }

    /// Get the command dispatcher for handling CLI commands
    pub fn dispatcher(&self) -> Arc<CommandDispatcher> {
        self.command_dispatcher.clone()
    }

    /// Get the workflow service for direct workflow operations
    pub fn workflows(&self) -> Arc<WorkflowService> {
        self.workflow_service.clone()
    }

    /// Get the config service for configuration operations
    pub fn config(&self) -> Arc<ConfigService> {
        self.config_service.clone()
    }

    /// Get the sync service for Git synchronization
    pub fn sync(&self) -> Arc<SyncService> {
        self.sync_service.clone()
    }
}
