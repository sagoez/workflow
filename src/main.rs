//! Event-driven workflow application entry point
//!
//! This version uses the proper event-driven architecture with:
//! - Command Dispatcher routing commands through 4-phase lifecycle
//! - Application Engine managing state through events
//! - Event Bus coordinating between commands and side effects
//! - No direct service calls - everything goes through events

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tokio::sync::Mutex;
use workflow::{
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
    shared::{
        AppConfig, ApplicationEngine, Cli, CommandContext, CommandDispatcher, EventBus, LoggingEventHandler,
        StatisticsEventHandler
    }
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Create command context
    let context = CommandContext::new();

    // Initialize the event-driven application engine
    let dispatcher = initialize_event_driven_engine().await?;

    // Route commands through the event-driven dispatcher
    dispatcher.dispatch_cli(&cli, &context).await?;

    Ok(())
}

/// Initialize the complete event-driven architecture
async fn initialize_event_driven_engine() -> Result<CommandDispatcher> {
    // Initialize application configuration
    let app_config = AppConfig::new()?;
    app_config.ensure_dirs_exist()?;

    // Infrastructure adapters
    let filesystem = Arc::new(LocalFileSystem::new()) as Arc<dyn FileSystem>;
    let git_client = Arc::new(Git2Client::new()) as Arc<dyn GitClient>;
    let _http_client = Arc::new(ReqwestClient::new()) as Arc<dyn HttpClient>;

    // UI adapters
    let prompter = Arc::new(TerminalPrompter::new()) as Arc<dyn Prompter>;
    let renderer = Arc::new(TerminalRenderer::new()) as Arc<dyn Renderer>;
    let progress = Arc::new(TerminalProgressIndicator::new()) as Arc<dyn ProgressIndicator>;

    // Storage adapters - properly shared RocksDB instance
    let event_store = Arc::new(RocksDbEventStore::new(&app_config.database_path)?);
    let shared_db = event_store.get_db();

    // All stores share the same database with different column families
    let history_store = Arc::new(RocksDbHistoryStore::new(shared_db)) as Arc<dyn HistoryStore>;
    let event_store = event_store as Arc<dyn EventStore>;

    // Config store (file-based, stores in config directory)
    let config_store = Arc::new(FileConfigStore::new(app_config.config_dir.clone())) as Arc<dyn ConfigStore>;

    // Services (used by event handlers for side effects)
    let _workflow_service = Arc::new(WorkflowService::new(
        filesystem.clone(),
        event_store.clone(),
        history_store.clone(),
        config_store.clone(),
        prompter.clone(),
        renderer.clone(),
        progress.clone()
    ));

    let _config_service = Arc::new(ConfigService::new(
        config_store.clone(),
        event_store.clone(),
        filesystem.clone(),
        renderer.clone(),
        progress.clone()
    ));

    let _sync_service = Arc::new(SyncService::new(
        config_store.clone(),
        event_store.clone(),
        git_client.clone(),
        filesystem.clone(),
        renderer.clone(),
        progress.clone()
    ));

    // Create event bus with handlers
    let mut event_bus = EventBus::new(event_store.clone());

    // Register event handlers for side effects
    event_bus.register_handler(Box::new(LoggingEventHandler::new()));
    event_bus.register_handler(Box::new(StatisticsEventHandler::new()));

    // TODO: Register service-specific event handlers that trigger side effects
    // e.g., WorkflowExecutionHandler that uses workflow_service
    // e.g., ConfigurationHandler that uses config_service
    // e.g., SyncHandler that uses sync_service

    let event_bus = Arc::new(event_bus);

    // Create application engine
    let engine = ApplicationEngine::new(event_bus.clone());
    let engine = Arc::new(Mutex::new(engine));

    // Create command dispatcher
    let dispatcher = CommandDispatcher::new(engine);

    Ok(dispatcher)
}
