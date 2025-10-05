pub mod actor;
pub mod adapter;
pub mod domain;
pub mod i18n;
pub mod port;
pub mod service;

use std::sync::Arc;

use crate::{
    adapter::git::Git2Client, domain::error::WorkflowError, i18n::display::TextManager, port::git::GitClient,
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
    pub event_store:  Arc<dyn crate::port::storage::EventStore>
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

        Ok(Self { config, text_manager: text_manager.clone(), git_client, event_store })
    }
}
