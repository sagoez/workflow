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
    pub git_client:   Arc<dyn GitClient>
}

impl AppContext {
    /// Creates a new app context with configuration from file (or defaults)
    pub fn init() -> Result<Self, WorkflowError> {
        // First create a temporary config to read the storage setting
        let temp_config = AppConfig::init()?;
        let storage_type = temp_config.get_current_storage()?;
        
        // Now create the real config with the correct storage type
        let config = AppConfig::with_storage_type(storage_type)?;
        config.ensure_dirs_exist()?;
        let text_manager = TextManager::init(Some(config.config_dir.clone()));
        let git_client = Arc::new(Git2Client::new()) as Arc<dyn GitClient>;

        Ok(Self { config, text_manager: text_manager.clone(), git_client })
    }

    /// Creates a new app context with specified storage type
    pub fn with_storage_type(storage_type: crate::adapter::storage::EventStoreType) -> Result<Self, WorkflowError> {
        let config = AppConfig::with_storage_type(storage_type)?;
        config.ensure_dirs_exist()?;
        let text_manager = TextManager::init(Some(config.config_dir.clone()));
        let git_client = Arc::new(Git2Client::new()) as Arc<dyn GitClient>;

        Ok(Self { config, text_manager: text_manager.clone(), git_client })
    }
}
