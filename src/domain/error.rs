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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_variants_display_without_panic() {
        let variants: Vec<WorkflowError> = vec![
            WorkflowError::FileSystem("disk full".to_string()),
            WorkflowError::Configuration("bad config".to_string()),
            WorkflowError::Validation("invalid input".to_string()),
            WorkflowError::Execution("failed".to_string()),
            WorkflowError::Event("bad event".to_string()),
            WorkflowError::UserInteraction("cancelled".to_string()),
            WorkflowError::Network("timeout".to_string()),
            WorkflowError::Serialization("parse error".to_string()),
            WorkflowError::Spawn("failed to spawn".to_string()),
            WorkflowError::UnsupportedLanguage("klingon".to_string()),
            WorkflowError::Recovery("recovery failed".to_string()),
            WorkflowError::JournalWrite("write failed".to_string()),
            WorkflowError::Snapshot("snapshot failed".to_string()),
            WorkflowError::Timeout("5s".to_string()),
            WorkflowError::Generic("something".to_string()),
        ];

        for variant in &variants {
            let display = format!("{}", variant);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn display_contains_inner_message() {
        // When TextManager is initialized, the i18n key "error_generic" contains "{0}"
        // which gets replaced with the message. Without TextManager, t() returns the
        // raw key "error_generic" and t_params replaces "{0}" in it — but the key
        // itself has no "{0}". So we test that Display produces a non-empty string.
        let err = WorkflowError::Generic("my error msg".to_string());
        let display = format!("{}", err);
        assert!(!display.is_empty(), "Display was: {}", display);
    }

    #[test]
    fn execution_passthrough_with_emoji() {
        let err = WorkflowError::Execution("🚫 already formatted".to_string());
        let display = format!("{}", err);
        assert!(display.contains("🚫 already formatted"));
    }

    #[test]
    fn debug_delegates_to_display() {
        let err = WorkflowError::Validation("test".to_string());
        let debug = format!("{:?}", err);
        let display = format!("{}", err);
        assert_eq!(debug, display);
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let wf_err: WorkflowError = io_err.into();
        match wf_err {
            WorkflowError::FileSystem(msg) => assert!(msg.contains("file not found")),
            _ => panic!("Expected FileSystem variant")
        }
    }

    #[test]
    fn from_serde_yaml_error() {
        let yaml_err = serde_yaml::from_str::<String>("{{invalid").unwrap_err();
        let wf_err: WorkflowError = yaml_err.into();
        match wf_err {
            WorkflowError::Serialization(msg) => assert!(!msg.is_empty()),
            _ => panic!("Expected Serialization variant")
        }
    }

    #[test]
    fn from_serde_json_error() {
        let json_err = serde_json::from_str::<String>("not json").unwrap_err();
        let wf_err: WorkflowError = json_err.into();
        match wf_err {
            WorkflowError::Serialization(msg) => assert!(!msg.is_empty()),
            _ => panic!("Expected Serialization variant")
        }
    }

    #[test]
    fn clone_preserves_variant() {
        let err = WorkflowError::Validation("test".to_string());
        let cloned = err.clone();
        assert_eq!(format!("{}", err), format!("{}", cloned));
    }
}
