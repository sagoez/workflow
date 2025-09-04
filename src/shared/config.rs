//! Application configuration
//!
//! This module provides application-wide configuration management
//! including database paths, storage settings, and other runtime configuration.

use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;

/// Application configuration for storage and runtime settings
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Base configuration directory
    pub config_dir:    PathBuf,
    /// Workflows directory
    pub workflows_dir: PathBuf,
    /// i18n directory
    pub i18n_dir:      PathBuf,
    /// Database file path
    pub database_path: PathBuf
}

impl AppConfig {
    /// Create a new application configuration with default paths
    pub fn new() -> Result<Self> {
        let config_dir = get_config_dir()?;
        let workflows_dir = config_dir.join("workflows");
        let i18n_dir = config_dir.join("i18n");
        let database_path = config_dir.join("workflow.db");

        Ok(Self { config_dir, workflows_dir, i18n_dir, database_path })
    }

    /// Create configuration directories if they don't exist
    pub fn ensure_dirs_exist(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.workflows_dir)?;
        std::fs::create_dir_all(&self.i18n_dir)?;
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new().expect("Failed to create default configuration")
    }
}

/// Get the project directories for cross-platform config path resolution
pub fn get_project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("", "", "workflow-rs").context("Failed to determine project directories")
}

/// Get the configuration directory path
pub fn get_config_dir() -> Result<PathBuf> {
    let project_dirs = get_project_dirs()?;
    Ok(project_dirs.config_dir().to_path_buf())
}

/// Get the workflows directory path
pub fn get_workflows_dir() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("workflows"))
}

/// Get the i18n directory path
pub fn get_i18n_dir() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("i18n"))
}

/// Initialize configuration directories
pub fn init_config_dirs() -> Result<()> {
    let config = AppConfig::new()?;
    config.ensure_dirs_exist()
}
