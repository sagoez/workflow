//! # New Event-Driven Workflow CLI Application
//!
//! This is the new main entry point that uses the event-driven architecture
//! with services, ports, and adapters.

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use workflow::{
    // CLI types
    Cli,
    Commands,
    LangCommands,
    ResourceCommands,
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
    shared::{AppConfig, CommandContext, Workflow, WorkflowError}
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Create command context
    let context = CommandContext::new();

    // Initialize services
    let services = initialize_services().await?;

    // Dispatch command
    match &cli.command {
        Some(Commands::Init) => {
            services.config.initialize_config(&context).await?;
        }
        Some(Commands::Lang { command }) => {
            handle_lang_command(command, &services.config, &context).await?;
        }
        Some(Commands::Resource { command }) => {
            handle_resource_command(command, &services.config, &context).await?;
        }
        Some(Commands::Sync { url, ssh_key }) => {
            if let Some(url) = url {
                services.sync.sync_from_url(url, ssh_key.as_deref(), &context).await?;
            } else {
                services.sync.sync_workflows(&context).await?;
            }
        }
        None => {
            if cli.list {
                services.workflow.list_workflows(&context).await?;
            } else if let Some(file_path) = &cli.file {
                // Resolve file path
                let full_path = resolve_workflow_path(file_path)?;
                services.workflow.execute_workflow(&full_path, &context).await?;
            } else {
                services.workflow.select_and_execute_workflow(&context).await?;
            }
        }
    }

    Ok(())
}

/// Services container
struct Services {
    workflow: Arc<WorkflowService>,
    config:   Arc<ConfigService>,
    sync:     Arc<SyncService>
}

/// Initialize all services with their dependencies
async fn initialize_services() -> Result<Services> {
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

    Ok(Services { workflow: workflow_service, config: config_service, sync: sync_service })
}

/// Handle language commands
async fn handle_lang_command(
    command: &LangCommands,
    config_service: &ConfigService,
    context: &CommandContext
) -> Result<(), WorkflowError> {
    match command {
        LangCommands::Set { language } => {
            config_service.set_language(language, context).await?;
        }
        LangCommands::List => {
            let languages = config_service.list_languages().await?;
            println!("Available languages:");
            for lang in languages {
                println!("  {}", lang);
            }
        }
        LangCommands::Current => {
            let current = config_service.get_current_language().await?;
            println!("Current language: {}", current);
        }
    }
    Ok(())
}

/// Handle resource commands
async fn handle_resource_command(
    command: &ResourceCommands,
    config_service: &ConfigService,
    context: &CommandContext
) -> Result<(), WorkflowError> {
    match command {
        ResourceCommands::Set { url } => {
            config_service.set_resource_url(url, context).await?;
        }
        ResourceCommands::Current => match config_service.get_resource_url().await? {
            Some(url) => println!("Current resource URL: {}", url),
            None => println!("No resource URL configured")
        }
    }
    Ok(())
}

/// Resolve workflow file path
fn resolve_workflow_path(file_path: &str) -> Result<String> {
    use std::path::Path;

    let path = Path::new(file_path);

    if path.is_absolute() || file_path.contains('/') {
        Ok(file_path.to_string())
    } else {
        // Look in the workflows config directory
        let app_config = AppConfig::new()?;
        Ok(app_config.workflows_dir.join(file_path).to_string_lossy().to_string())
    }
}
