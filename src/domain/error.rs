use std::fmt;

use thiserror::Error;

use crate::t_params;

#[derive(Debug, Clone, Error)]
pub enum ValidationError {
    InvalidState(String),
    ArgumentNotResolved(String),
    SelectionFailed(String, String),
    InputFailed(String, String),
    EnumMissingConfig(String),
    DynamicResolutionFailed(String),
    NoOptionsFound(String),
    Other(String)
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState(msg) => write!(f, "{}", t_params!("error_validation", &[msg])),
            Self::ArgumentNotResolved(name) => {
                write!(f, "{}", t_params!("error_argument_not_resolved", &[name]))
            }
            Self::SelectionFailed(target, msg) => {
                write!(f, "{}", t_params!("error_selection_failed", &[target, msg]))
            }
            Self::InputFailed(target, msg) => {
                write!(f, "{}", t_params!("error_input_failed", &[target, msg]))
            }
            Self::EnumMissingConfig(name) => {
                write!(f, "{}", t_params!("error_enum_argument_missing_configuration", &[name]))
            }
            Self::DynamicResolutionFailed(name) => {
                write!(f, "{}", t_params!("error_dynamic_resolution_failed", &[name]))
            }
            Self::NoOptionsFound(name) => {
                write!(f, "{}", t_params!("error_no_options_found", &[name]))
            }
            Self::Other(msg) => write!(f, "{}", t_params!("error_validation", &[msg]))
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum StorageError {
    Io(String),
    Serialization(String),
    NotFound(String)
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "{}", t_params!("error_file_system", &[msg])),
            Self::Serialization(msg) => write!(f, "{}", t_params!("error_serialization", &[msg])),
            Self::NotFound(id) => write!(f, "{}", t_params!("storage_aggregate_not_found", &[id]))
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum PromptError {
    Interaction(String)
}

impl fmt::Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interaction(msg) => write!(f, "{}", t_params!("error_user_interaction", &[msg]))
        }
    }
}

#[derive(Clone)]
pub enum WorkflowError {
    Validation(ValidationError),
    Storage(StorageError),
    Prompt(PromptError),
    Cancelled,
    Execution(String),
    Network(String),
    Spawn(String),
    Timeout(String),
    Config(String),
    Other(String)
}

impl std::error::Error for WorkflowError {}

impl WorkflowError {
    pub fn wrap(self, f: impl FnOnce(String) -> WorkflowError) -> WorkflowError {
        match self {
            WorkflowError::Cancelled => WorkflowError::Cancelled,
            other => f(other.to_string())
        }
    }
}

impl From<ValidationError> for WorkflowError {
    fn from(err: ValidationError) -> Self {
        WorkflowError::Validation(err)
    }
}

impl From<StorageError> for WorkflowError {
    fn from(err: StorageError) -> Self {
        WorkflowError::Storage(err)
    }
}

impl From<PromptError> for WorkflowError {
    fn from(err: PromptError) -> Self {
        WorkflowError::Prompt(err)
    }
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(e) => write!(f, "{}", e),
            Self::Storage(e) => write!(f, "{}", e),
            Self::Prompt(e) => write!(f, "{}", e),
            Self::Cancelled => Ok(()),
            Self::Execution(msg) => write!(f, "{}", t_params!("error_execution", &[msg])),
            Self::Network(msg) => write!(f, "{}", t_params!("error_network", &[msg])),
            Self::Spawn(msg) => write!(f, "{}", t_params!("error_spawn", &[msg])),
            Self::Timeout(msg) => write!(f, "{}", t_params!("error_timeout", &[msg])),
            Self::Config(msg) => write!(f, "{}", t_params!("error_configuration", &[msg])),
            Self::Other(msg) => write!(f, "{}", t_params!("error_generic", &[msg]))
        }
    }
}

impl fmt::Debug for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::Io(err.to_string())
    }
}

impl From<std::io::Error> for WorkflowError {
    fn from(err: std::io::Error) -> Self {
        WorkflowError::Storage(StorageError::Io(err.to_string()))
    }
}

impl From<serde_yaml::Error> for WorkflowError {
    fn from(err: serde_yaml::Error) -> Self {
        WorkflowError::Storage(StorageError::Serialization(err.to_string()))
    }
}

impl From<serde_json::Error> for WorkflowError {
    fn from(err: serde_json::Error) -> Self {
        WorkflowError::Storage(StorageError::Serialization(err.to_string()))
    }
}

impl From<ractor::SpawnErr> for WorkflowError {
    fn from(err: ractor::SpawnErr) -> Self {
        WorkflowError::Spawn(err.to_string())
    }
}

impl From<anyhow::Error> for WorkflowError {
    fn from(err: anyhow::Error) -> Self {
        WorkflowError::Other(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelled_variant_exists() {
        let err = WorkflowError::Cancelled;
        assert!(matches!(err, WorkflowError::Cancelled));
    }

    #[test]
    fn cancelled_display_is_empty() {
        let err = WorkflowError::Cancelled;
        assert_eq!(format!("{}", err), "");
    }

    #[test]
    fn validation_from_converts() {
        let v = ValidationError::ArgumentNotResolved("port".to_string());
        let err: WorkflowError = v.into();
        assert!(matches!(err, WorkflowError::Validation(_)));
    }

    #[test]
    fn storage_from_converts() {
        let s = StorageError::Io("disk full".to_string());
        let err: WorkflowError = s.into();
        assert!(matches!(err, WorkflowError::Storage(_)));
    }

    #[test]
    fn prompt_from_converts() {
        let p = PromptError::Interaction("cancelled".to_string());
        let err: WorkflowError = p.into();
        assert!(matches!(err, WorkflowError::Prompt(_)));
    }

    #[test]
    fn io_error_converts_to_storage() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: WorkflowError = io_err.into();
        assert!(matches!(err, WorkflowError::Storage(StorageError::Io(_))));
    }

    #[test]
    fn serde_yaml_converts_to_storage() {
        let yaml_err = serde_yaml::from_str::<String>("{{invalid").unwrap_err();
        let err: WorkflowError = yaml_err.into();
        assert!(matches!(err, WorkflowError::Storage(StorageError::Serialization(_))));
    }

    #[test]
    fn serde_json_converts_to_storage() {
        let json_err = serde_json::from_str::<String>("not json").unwrap_err();
        let err: WorkflowError = json_err.into();
        assert!(matches!(err, WorkflowError::Storage(StorageError::Serialization(_))));
    }

    #[test]
    fn all_variants_display_without_panic() {
        let variants: Vec<WorkflowError> = vec![
            ValidationError::Other("bad".to_string()).into(),
            StorageError::Io("disk".to_string()).into(),
            PromptError::Interaction("cancelled".to_string()).into(),
            WorkflowError::Cancelled,
            WorkflowError::Execution("failed".to_string()),
            WorkflowError::Network("timeout".to_string()),
            WorkflowError::Spawn("failed".to_string()),
            WorkflowError::Timeout("5s".to_string()),
            WorkflowError::Config("bad".to_string()),
            WorkflowError::Other("something".to_string()),
        ];
        for variant in &variants {
            let _ = format!("{}", variant);
        }
    }

    #[test]
    fn debug_delegates_to_display() {
        let err: WorkflowError = ValidationError::Other("test".to_string()).into();
        assert_eq!(format!("{:?}", err), format!("{}", err));
    }

    #[test]
    fn clone_preserves_variant() {
        let err = WorkflowError::Execution("test".to_string());
        let cloned = err.clone();
        assert_eq!(format!("{}", err), format!("{}", cloned));
    }
}
