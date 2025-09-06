use thiserror::Error;

/// Common error types for the workflow system
#[derive(Error, Debug, Clone)]
pub enum WorkflowError {
    /// File system related errors
    #[error("{0}")]
    FileSystem(String),

    /// Configuration related errors
    #[error("{0}")]
    Configuration(String),

    /// Validation errors
    #[error("{0}")]
    Validation(String),

    /// Command execution errors
    #[error("{0}")]
    Execution(String),

    /// Event processing errors
    #[error("{0}")]
    Event(String),

    /// User interaction errors
    #[error("{0}")]
    UserInteraction(String),

    /// Network/IO errors
    #[error("{0}")]
    Network(String),

    /// Serialization/deserialization errors
    #[error("{0}")]
    Serialization(String),

    /// Spawn errors
    #[error("{0}")]
    Spawn(String),

    /// Unsupported language
    #[error("{0}")]
    UnsupportedLanguage(String),

    /// Recovery errors
    #[error("{0}")]
    Recovery(String),

    /// Journal write errors
    #[error("{0}")]
    JournalWrite(String),

    /// Snapshot errors
    #[error("{0}")]
    Snapshot(String),

    /// Timeout errors
    #[error("{0}")]
    Timeout(String),

    /// Generic errors with context
    #[error("{0}")]
    Generic(String)
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
