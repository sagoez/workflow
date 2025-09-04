//! Common error types used throughout the system

use thiserror::Error;
use crate::i18n;

/// Common error types for the workflow system
#[derive(Error, Debug, Clone)]
pub enum WorkflowError {
    /// File system related errors
    #[error("error_filesystem")]
    FileSystem(String),

    /// Configuration related errors  
    #[error("error_configuration")]
    Configuration(String),

    /// Validation errors
    #[error("error_validation")]
    Validation(String),

    /// Command execution errors
    #[error("error_execution")]
    Execution(String),

    /// Event processing errors
    #[error("error_event")]
    Event(String),

    /// User interaction errors
    #[error("error_user_interaction")]
    UserInteraction(String),

    /// Network/IO errors
    #[error("error_network")]
    Network(String),

    /// Serialization/deserialization errors
    #[error("error_serialization")]
    Serialization(String),

    /// Generic errors with context
    #[error("error_generic")]
    Generic(String),
}

impl WorkflowError {
    /// Get the localized error message
    pub fn localized_message(&self) -> String {
        use crate::i18n;
        
        match self {
            WorkflowError::FileSystem(_) => i18n::t("error_filesystem"),
            WorkflowError::Configuration(msg) => i18n::t_params("error_configuration", &[msg]),
            WorkflowError::Validation(msg) => i18n::t_params("error_validation", &[msg]),
            WorkflowError::Execution(msg) => i18n::t_params("error_execution", &[msg]),
            WorkflowError::Event(msg) => i18n::t_params("error_event", &[msg]),
            WorkflowError::UserInteraction(msg) => i18n::t_params("error_user_interaction", &[msg]),
            WorkflowError::Network(msg) => i18n::t_params("error_network", &[msg]),
            WorkflowError::Serialization(msg) => i18n::t_params("error_serialization", &[msg]),
            WorkflowError::Generic(msg) => i18n::t_params("error_generic", &[msg]),
        }
    }

    // Database error helpers
    pub fn db_failed_to_open_rocksdb(error: impl std::fmt::Display) -> Self {
        WorkflowError::Configuration(i18n::t_params("db_failed_to_open_rocksdb", &[&error.to_string()]))
    }

    pub fn db_column_family_not_found(name: &str) -> Self {
        WorkflowError::Configuration(i18n::t_params("db_column_family_not_found", &[name]))
    }

    pub fn db_failed_to_save_event(error: impl std::fmt::Display) -> Self {
        WorkflowError::Event(i18n::t_params("db_failed_to_save_event", &[&error.to_string()]))
    }

    pub fn db_failed_to_read_event(error: impl std::fmt::Display) -> Self {
        WorkflowError::Event(i18n::t_params("db_failed_to_read_event", &[&error.to_string()]))
    }

    pub fn db_failed_to_load_config(error: impl std::fmt::Display) -> Self {
        WorkflowError::Configuration(i18n::t_params("db_failed_to_load_config", &[&error.to_string()]))
    }

    pub fn db_invalid_utf8_in_config(error: impl std::fmt::Display) -> Self {
        WorkflowError::Configuration(i18n::t_params("db_invalid_utf8_in_config", &[&error.to_string()]))
    }

    pub fn db_failed_to_parse_config_json(error: impl std::fmt::Display) -> Self {
        WorkflowError::Configuration(i18n::t_params("db_failed_to_parse_config_json", &[&error.to_string()]))
    }

    pub fn db_failed_to_serialize_config(error: impl std::fmt::Display) -> Self {
        WorkflowError::Configuration(i18n::t_params("db_failed_to_serialize_config", &[&error.to_string()]))
    }

    pub fn db_failed_to_save_config(error: impl std::fmt::Display) -> Self {
        WorkflowError::Configuration(i18n::t_params("db_failed_to_save_config", &[&error.to_string()]))
    }

    // File system error helpers
    pub fn fs_failed_to_create_i18n_dir(error: impl std::fmt::Display) -> Self {
        WorkflowError::Configuration(i18n::t_params("fs_failed_to_create_i18n_dir", &[&error.to_string()]))
    }

    pub fn fs_failed_to_write_en_translations(error: impl std::fmt::Display) -> Self {
        WorkflowError::Configuration(i18n::t_params("fs_failed_to_write_en_translations", &[&error.to_string()]))
    }

    pub fn fs_failed_to_write_es_translations(error: impl std::fmt::Display) -> Self {
        WorkflowError::Configuration(i18n::t_params("fs_failed_to_write_es_translations", &[&error.to_string()]))
    }

    pub fn fs_failed_to_read_config_file(error: impl std::fmt::Display) -> Self {
        WorkflowError::Configuration(i18n::t_params("fs_failed_to_read_config_file", &[&error.to_string()]))
    }

    pub fn fs_failed_to_parse_config_file(error: impl std::fmt::Display) -> Self {
        WorkflowError::Configuration(i18n::t_params("fs_failed_to_parse_config_file", &[&error.to_string()]))
    }

    // Git error helpers
    pub fn git_failed_to_clone_repository(error: impl std::fmt::Display) -> Self {
        WorkflowError::Network(i18n::t_params("git_failed_to_clone_repository", &[&error.to_string()]))
    }

    pub fn git_failed_to_open_repository(error: impl std::fmt::Display) -> Self {
        WorkflowError::Configuration(i18n::t_params("git_failed_to_open_repository", &[&error.to_string()]))
    }

    pub fn git_failed_to_find_remote(remote_name: &str, error: impl std::fmt::Display) -> Self {
        WorkflowError::Network(i18n::t_params("git_failed_to_find_remote", &[remote_name, &error.to_string()]))
    }

    pub fn git_invalid_commit_id(error: impl std::fmt::Display) -> Self {
        WorkflowError::Validation(i18n::t_params("git_invalid_commit_id", &[&error.to_string()]))
    }

    pub fn git_remote_has_no_url(remote_name: &str) -> Self {
        WorkflowError::Network(i18n::t_params("git_remote_has_no_url", &[remote_name]))
    }

    // Network error helpers
    pub fn net_get_request_failed(error: impl std::fmt::Display) -> Self {
        WorkflowError::Network(i18n::t_params("net_get_request_failed", &[&error.to_string()]))
    }

    pub fn net_failed_to_read_response_body(error: impl std::fmt::Display) -> Self {
        WorkflowError::Network(i18n::t_params("net_failed_to_read_response_body", &[&error.to_string()]))
    }

    pub fn net_post_request_failed(error: impl std::fmt::Display) -> Self {
        WorkflowError::Network(i18n::t_params("net_post_request_failed", &[&error.to_string()]))
    }

    pub fn net_download_failed_with_status(status: impl std::fmt::Display) -> Self {
        WorkflowError::Network(i18n::t_params("net_download_failed_with_status", &[&status.to_string()]))
    }
}

/// Convert from anyhow::Error
impl From<anyhow::Error> for WorkflowError {
    fn from(err: anyhow::Error) -> Self {
        WorkflowError::Generic(err.to_string())
    }
}

/// Convert from std::io::Error
impl From<std::io::Error> for WorkflowError {
    fn from(err: std::io::Error) -> Self {
        WorkflowError::FileSystem(err.to_string())
    }
}

/// Convert from serde_yaml::Error
impl From<serde_yaml::Error> for WorkflowError {
    fn from(err: serde_yaml::Error) -> Self {
        WorkflowError::Serialization(err.to_string())
    }
}

/// Convert from serde_json::Error
impl From<serde_json::Error> for WorkflowError {
    fn from(err: serde_json::Error) -> Self {
        WorkflowError::Serialization(err.to_string())
    }
}
