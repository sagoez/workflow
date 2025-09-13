use std::fmt;

use thiserror::Error;

use crate::t_params;

/// Common error types for the workflow system
#[derive(Clone, Error)]
pub enum WorkflowError {
    /// File system related errors
    FileSystem(String),

    /// Configuration related errors
    Configuration(String),

    /// Validation errors
    Validation(String),

    /// Command execution errors
    Execution(String),

    /// Event processing errors
    Event(String),

    /// User interaction errors
    UserInteraction(String),

    /// Network/IO errors
    Network(String),

    /// Serialization/deserialization errors
    Serialization(String),

    /// Spawn errors
    Spawn(String),

    /// Unsupported language
    UnsupportedLanguage(String),

    /// Recovery errors
    Recovery(String),

    /// Journal write errors
    JournalWrite(String),

    /// Snapshot errors
    Snapshot(String),

    /// Timeout errors
    Timeout(String),

    /// Generic errors with context
    Generic(String)
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            WorkflowError::FileSystem(msg) => t_params!("error_file_system", &[msg]),
            WorkflowError::Configuration(msg) => t_params!("error_configuration", &[msg]),
            WorkflowError::Validation(msg) => t_params!("error_validation", &[msg]),
            WorkflowError::Execution(msg) => {
                if msg.contains("🚫") || msg.contains("❌") || msg.contains("📁") || msg.contains("⚙️") {
                    msg.clone()
                } else {
                    t_params!("error_execution", &[msg])
                }
            }
            WorkflowError::Event(msg) => t_params!("error_event", &[msg]),
            WorkflowError::UserInteraction(msg) => t_params!("error_user_interaction", &[msg]),
            WorkflowError::Network(msg) => t_params!("error_network", &[msg]),
            WorkflowError::Serialization(msg) => t_params!("error_serialization", &[msg]),
            WorkflowError::Spawn(msg) => t_params!("error_spawn", &[msg]),
            WorkflowError::UnsupportedLanguage(msg) => t_params!("error_unsupported_language", &[msg]),
            WorkflowError::Recovery(msg) => t_params!("error_recovery", &[msg]),
            WorkflowError::JournalWrite(msg) => t_params!("error_journal_write", &[msg]),
            WorkflowError::Snapshot(msg) => t_params!("error_snapshot", &[msg]),
            WorkflowError::Timeout(msg) => t_params!("error_timeout", &[msg]),
            WorkflowError::Generic(msg) => t_params!("error_generic", &[msg])
        };

        if msg.contains('\n') && msg.contains("└─") {
            return write!(f, "{}", msg);
        }

        if msg.contains('\n') && msg.lines().any(|line| line.trim().starts_with("  ")) {
            return write!(f, "{}", msg);
        }

        let parts: Vec<&str> = msg.split(':').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

        if parts.len() <= 1 {
            write!(f, "{}", msg)
        } else {
            let mut result = vec![parts[0].to_string()];
            for (i, part) in parts.iter().enumerate().skip(1) {
                let indent = "  ".repeat(i);
                result.push(format!("{}└─ {}", indent, part));
            }
            write!(f, "{}", result.join("\n"))
        }
    }
}

impl fmt::Debug for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
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

/// Convert from ractor::SpawnErr
impl From<ractor::SpawnErr> for WorkflowError {
    fn from(err: ractor::SpawnErr) -> Self {
        WorkflowError::Spawn(err.to_string())
    }
}
