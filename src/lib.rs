pub mod actor;
pub mod adapter;
pub mod domain;
pub mod i18n;
pub mod port;
pub mod service;

use std::sync::Arc;

use crate::{
    adapter::{
        executor::ShellExecutor, filesystem::StdFileSystem, git::Git2Client, output::CliOutput, prompt::CliPrompt
    },
    domain::error::WorkflowError,
    i18n::display::TextManager,
    port::{
        executor::CommandExecutor, filesystem::FileSystem, git::GitClient, output::OutputWriter, prompt::UserPrompt
    },
    service::config::AppConfig
};

/// Application context for dependency injection
#[derive(Clone)]
pub struct AppContext {
    /// Application configuration
    pub config:       AppConfig,
    /// Text manager
    pub text_manager: TextManager,
    /// Git client for repository operations
    pub git_client:   Arc<dyn GitClient>,
    /// Event store for persistence
    pub event_store:  Arc<dyn crate::port::storage::EventStore>,
    /// User prompt for interactive input
    pub prompt:       Arc<dyn UserPrompt>,
    /// Command executor for shell commands
    pub executor:     Arc<dyn CommandExecutor>,
    /// File system operations
    pub filesystem:   Arc<dyn FileSystem>,
    /// Output writer for CLI display
    pub output:       Arc<dyn OutputWriter>
}

impl AppContext {
    /// Creates a new app context with configuration from file (or defaults)
    ///
    /// Initializes the application with:
    /// - Configuration from file or defaults
    /// - Text manager for i18n
    /// - Git client for repository operations
    /// - Event store with shared RocksDB instance for Journal/EventStore coordination
    pub fn init() -> Result<Self, WorkflowError> {
        let temp_config = AppConfig::init()?;
        let storage_type = temp_config.get_current_storage()?;

        let config = AppConfig::with_storage_type(storage_type)?;
        config.ensure_dirs_exist()?;
        let text_manager = TextManager::init(Some(config.config_dir.clone()));
        let git_client = Arc::new(Git2Client::new()) as Arc<dyn GitClient>;
        let event_store =
            crate::adapter::storage::EventStoreFactory::create(config.storage_type, Some(&config.database_path))?;

        let prompt = Arc::new(CliPrompt::new()) as Arc<dyn UserPrompt>;
        let executor = Arc::new(ShellExecutor::new()) as Arc<dyn CommandExecutor>;
        let filesystem = Arc::new(StdFileSystem::new()) as Arc<dyn FileSystem>;
        let output = Arc::new(CliOutput::default()) as Arc<dyn OutputWriter>;

        Ok(Self {
            config,
            text_manager: text_manager.clone(),
            git_client,
            event_store,
            prompt,
            executor,
            filesystem,
            output
        })
    }
}
