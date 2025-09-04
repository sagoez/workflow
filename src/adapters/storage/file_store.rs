//! File-based implementation of storage ports

use std::path::PathBuf;

use async_trait::async_trait;

use crate::{
    ports::storage::{Config, ConfigStore},
    shared::WorkflowError
};

/// File-based implementation of ConfigStore
pub struct FileConfigStore {
    config_dir: PathBuf
}

impl FileConfigStore {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    fn get_config_file_path(&self) -> PathBuf {
        self.config_dir.join("config.yaml")
    }

    fn get_i18n_dir(&self) -> PathBuf {
        self.config_dir.join("i18n")
    }

    /// Copy default translation files to the user's config directory
    async fn copy_default_translations(&self) -> Result<(), WorkflowError> {
        let i18n_dir = self.get_i18n_dir();

        // Ensure i18n directory exists
        tokio::fs::create_dir_all(&i18n_dir).await.map_err(WorkflowError::fs_failed_to_create_i18n_dir)?;

        // Write English translations
        let en_file = i18n_dir.join("en.yaml");
        let en_content = include_str!("../../../config/i18n/en.yaml");
        tokio::fs::write(&en_file, en_content).await.map_err(WorkflowError::fs_failed_to_write_en_translations)?;

        // Write Spanish translations
        let es_file = i18n_dir.join("es.yaml");
        let es_content = include_str!("../../../config/i18n/es.yaml");
        tokio::fs::write(&es_file, es_content).await.map_err(WorkflowError::fs_failed_to_write_es_translations)?;

        Ok(())
    }
}

#[async_trait]
impl ConfigStore for FileConfigStore {
    async fn load_config(&self) -> Result<Config, WorkflowError> {
        let config_path = self.get_config_file_path();

        if !config_path.exists() {
            // Return default config if file doesn't exist
            return Ok(Config::default());
        }

        let content =
            tokio::fs::read_to_string(&config_path).await.map_err(WorkflowError::fs_failed_to_read_config_file)?;

        serde_yaml::from_str(&content).map_err(WorkflowError::fs_failed_to_parse_config_file)
    }

    async fn save_config(&self, config: &Config) -> Result<(), WorkflowError> {
        let config_path = self.get_config_file_path();

        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| WorkflowError::Configuration(format!("Failed to create config directory: {}", e)))?;
        }

        let content = serde_yaml::to_string(config)
            .map_err(|e| WorkflowError::Configuration(format!("Failed to serialize config: {}", e)))?;

        tokio::fs::write(&config_path, content)
            .await
            .map_err(|e| WorkflowError::Configuration(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    async fn config_exists(&self) -> Result<bool, WorkflowError> {
        let config_path = self.get_config_file_path();
        Ok(config_path.exists())
    }

    async fn init_config(&self) -> Result<(), WorkflowError> {
        let config_dir = &self.config_dir;

        // Create config directory
        tokio::fs::create_dir_all(config_dir)
            .await
            .map_err(|e| WorkflowError::Configuration(format!("Failed to create config directory: {}", e)))?;

        // Copy default translations
        self.copy_default_translations().await?;

        // Create default config if it doesn't exist
        if !self.config_exists().await? {
            let default_config = Config::default();
            self.save_config(&default_config).await?;
        }

        Ok(())
    }
}
