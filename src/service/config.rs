use std::{fs, path::PathBuf};

use directories::ProjectDirs;

use crate::{adapter::storage::EventStoreType, domain::error::WorkflowError, i18n::Language, t};

/// Application configuration for storage and runtime settings
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Base configuration directory
    pub config_dir:    PathBuf,
    /// Workflows directory
    pub workflows_dir: PathBuf,
    /// i18n directory
    pub i18n_dir:      PathBuf,
    /// Database file path (for RocksDB)
    pub database_path: PathBuf,
    /// Storage backend type
    pub storage_type:  EventStoreType
}

// TODO: All the fs operations are all over the place, we should move them to the a trait
// so they can be easily mocked for testing
// TODO: The memory type should also come from the config
impl AppConfig {
    pub fn init() -> Result<Self, WorkflowError> {
        let config = Self::new()?;
        config.ensure_dirs_exist()?;
        Ok(config)
    }

    /// Creates a new app config with default storage backend
    fn new() -> Result<Self, WorkflowError> {
        Self::with_storage_type(EventStoreType::RocksDb)
    }

    /// Creates a new app config with specified storage backend
    pub fn with_storage_type(storage_type: EventStoreType) -> Result<Self, WorkflowError> {
        let project_dirs =
            ProjectDirs::from("org", "sagoez", "workflow").ok_or(WorkflowError::FileSystem(t!("error_filesystem")))?;

        let config_dir = project_dirs.config_dir().to_path_buf();
        let workflows_dir = config_dir.join("workflows");
        let i18n_dir = config_dir.join("i18n");
        let database_path = config_dir.join("rocksdb");

        Ok(Self { config_dir, workflows_dir, i18n_dir, database_path, storage_type })
    }

    /// Create configuration directories if they don't exist
    pub fn ensure_dirs_exist(&self) -> Result<(), WorkflowError> {
        fs::create_dir_all(&self.config_dir).map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
        fs::create_dir_all(&self.workflows_dir).map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
        fs::create_dir_all(&self.i18n_dir).map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
        Ok(())
    }

    /// Get the current language setting from config file
    pub fn get_current_language(&self) -> Result<Language, WorkflowError> {
        let lang_file = self.config_dir.join("language.txt");

        if lang_file.exists() {
            let content = fs::read_to_string(&lang_file)
                .map_err(|e| WorkflowError::FileSystem(format!("Failed to read language config: {}", e)))?;
            let lang_code = content.trim();
            Language::try_from(lang_code)
        } else {
            // Default to English if no config file exists
            Ok(Language::English)
        }
    }

    /// Set the current language setting in config file
    pub fn set_current_language(&self, language: Language) -> Result<(), WorkflowError> {
        let lang_file = self.config_dir.join("language.txt");
        fs::write(&lang_file, language.code())
            .map_err(|e| WorkflowError::FileSystem(format!("Failed to write language config: {}", e)))?;
        Ok(())
    }

    /// Get the current storage backend setting from config file
    pub fn get_current_storage(&self) -> Result<EventStoreType, WorkflowError> {
        let storage_file = self.config_dir.join("storage.txt");

        if storage_file.exists() {
            let content = fs::read_to_string(&storage_file)
                .map_err(|e| WorkflowError::FileSystem(format!("Failed to read storage config: {}", e)))?;
            EventStoreType::from_str(content.trim()).map_err(|e| WorkflowError::Validation(e))
        } else {
            // Default to RocksDB if no config file exists
            Ok(EventStoreType::RocksDb)
        }
    }

    /// Set the current storage backend setting in config file
    pub fn set_current_storage(&self, storage_type: EventStoreType) -> Result<(), WorkflowError> {
        let storage_file = self.config_dir.join("storage.txt");
        fs::write(&storage_file, storage_type.as_str())
            .map_err(|e| WorkflowError::FileSystem(format!("Failed to write storage config: {}", e)))?;
        Ok(())
    }
}
