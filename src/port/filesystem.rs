use std::path::{Path, PathBuf};

use crate::domain::error::WorkflowError;

/// Port trait for filesystem operations
pub trait FileSystem: Send + Sync {
    fn read_to_string(&self, path: &Path) -> Result<String, WorkflowError>;
    fn write(&self, path: &Path, contents: &str) -> Result<(), WorkflowError>;
    fn exists(&self, path: &Path) -> bool;
    fn create_dir_all(&self, path: &Path) -> Result<(), WorkflowError>;
    fn remove_file(&self, path: &Path) -> Result<(), WorkflowError>;
    fn remove_dir_all(&self, path: &Path) -> Result<(), WorkflowError>;
    fn read_dir_entries(&self, path: &Path) -> Result<Vec<PathBuf>, WorkflowError>;
}
