use std::{
    fs,
    path::{Path, PathBuf}
};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::i18n;

/// Configuration structure for the workflow CLI
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Currently selected language
    pub language:     String,
    /// Git URL for workflows repository
    pub resource_url: Option<String>
}

impl Default for Config {
    fn default() -> Self {
        Self { language: "en".to_string(), resource_url: None }
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

/// Get the i18n directory path
pub fn get_i18n_dir() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("i18n"))
}

/// Get the workflows directory path
pub fn get_workflows_dir() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("workflows"))
}

/// Get the config file path
pub fn get_config_file_path() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("config.yaml"))
}

/// Load configuration from file or create default if it doesn't exist
pub fn load_config() -> Result<Config> {
    let config_path = get_config_file_path()?;

    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .with_context(|| i18n::t_params("config_failed_to_read", &[&config_path.display().to_string()]))?;

        serde_yaml::from_str(&content).with_context(|| "Failed to parse config file")
    } else {
        let config = Config::default();
        save_config(&config)?;
        Ok(config)
    }
}

/// Save configuration to file
pub fn save_config(config: &Config) -> Result<()> {
    let config_path = get_config_file_path()?;

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| i18n::t_params("config_failed_to_create_dir", &[&parent.display().to_string()]))?;
    }

    let content = serde_yaml::to_string(config).context("Failed to serialize config")?;

    fs::write(&config_path, content)
        .with_context(|| i18n::t_params("config_failed_to_write", &[&config_path.display().to_string()]))?;

    Ok(())
}

/// Initialize the configuration directories and copy default files
pub fn init_config_dirs() -> Result<()> {
    let config_dir = get_config_dir()?;
    let i18n_dir = get_i18n_dir()?;

    fs::create_dir_all(&config_dir)
        .with_context(|| i18n::t_params("config_failed_to_create_dir", &[&config_dir.display().to_string()]))?;

    fs::create_dir_all(&i18n_dir)
        .with_context(|| i18n::t_params("config_failed_to_create_i18n_dir", &[&i18n_dir.display().to_string()]))?;

    copy_default_translations(&i18n_dir)?;

    if !get_config_file_path()?.exists() {
        save_config(&Config::default())?;
    }

    Ok(())
}

/// Copy default translation files to the user's config directory
/// Always updates existing files to ensure new translation keys are available
fn copy_default_translations(i18n_dir: &Path) -> Result<()> {
    let en_file = i18n_dir.join("en.yaml");
    let en_content = include_str!("../../config/i18n/en.yaml");
    fs::write(&en_file, en_content)
        .with_context(|| i18n::t_params("config_failed_to_write_en_translations", &[&en_file.display().to_string()]))?;

    let es_file = i18n_dir.join("es.yaml");
    let es_content = include_str!("../../config/i18n/es.yaml");
    fs::write(&es_file, es_content)
        .with_context(|| i18n::t_params("config_failed_to_write_es_translations", &[&es_file.display().to_string()]))?;

    Ok(())
}

/// Set the language in configuration
pub fn set_language(language: &str) -> Result<()> {
    let mut config = load_config()?;
    config.language = language.to_string();
    save_config(&config)?;
    Ok(())
}

/// Get the currently configured language
pub fn get_current_language() -> Result<String> {
    let config = load_config()?;
    Ok(config.language)
}

/// List available languages based on translation files in the i18n directory
pub fn list_available_languages() -> Result<Vec<String>> {
    let i18n_dir = get_i18n_dir()?;

    if !i18n_dir.exists() {
        init_config_dirs()?;
    }

    let mut languages = Vec::new();

    if let Ok(entries) = fs::read_dir(&i18n_dir) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str()
                && (file_name.ends_with(".yaml") || file_name.ends_with(".yml"))
                && let Some(lang_code) = file_name.strip_suffix(".yaml").or_else(|| file_name.strip_suffix(".yml"))
            {
                languages.push(lang_code.to_string());
            }
        }
    }

    languages.sort();
    Ok(languages)
}

/// Set the resource URL in configuration
pub fn set_resource_url(resource_url: &str) -> Result<()> {
    let mut config = load_config()?;
    config.resource_url = Some(resource_url.to_string());
    save_config(&config)?;
    Ok(())
}

/// Get the currently configured resource URL
pub fn get_current_resource_url() -> Result<Option<String>> {
    let config = load_config()?;
    Ok(config.resource_url)
}
