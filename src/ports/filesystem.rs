//! Filesystem ports - interfaces for file operations

use async_trait::async_trait;
use std::path::{Path, PathBuf};

use crate::shared::WorkflowError;

/// Port for file system operations
#[async_trait]
pub trait FileSystem: Send + Sync {
    /// Read file contents as string
    async fn read_to_string(&self, path: &Path) -> Result<String, WorkflowError>;
    
    /// Write string contents to file
    async fn write_string(&self, path: &Path, contents: &str) -> Result<(), WorkflowError>;
    
    /// Read file contents as bytes
    async fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, WorkflowError>;
    
    /// Write bytes to file
    async fn write_bytes(&self, path: &Path, contents: &[u8]) -> Result<(), WorkflowError>;
    
    /// Check if file exists
    async fn exists(&self, path: &Path) -> Result<bool, WorkflowError>;
    
    /// Check if path is a file
    async fn is_file(&self, path: &Path) -> Result<bool, WorkflowError>;
    
    /// Check if path is a directory
    async fn is_dir(&self, path: &Path) -> Result<bool, WorkflowError>;
    
    /// Create directory (and parent directories if needed)
    async fn create_dir_all(&self, path: &Path) -> Result<(), WorkflowError>;
    
    /// Remove file
    async fn remove_file(&self, path: &Path) -> Result<(), WorkflowError>;
    
    /// Remove directory and all contents
    async fn remove_dir_all(&self, path: &Path) -> Result<(), WorkflowError>;
    
    /// List directory contents
    async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, WorkflowError>;
    
    /// Copy file from source to destination
    async fn copy_file(&self, from: &Path, to: &Path) -> Result<(), WorkflowError>;
    
    /// Move/rename file
    async fn move_file(&self, from: &Path, to: &Path) -> Result<(), WorkflowError>;
    
    /// Get file metadata
    async fn metadata(&self, path: &Path) -> Result<FileMetadata, WorkflowError>;
    
    /// Find files matching a pattern
    async fn find_files(&self, dir: &Path, pattern: &str) -> Result<Vec<PathBuf>, WorkflowError>;
}

/// File metadata information
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub modified: Option<std::time::SystemTime>,
    pub created: Option<std::time::SystemTime>,
    pub permissions: FilePermissions,
}

/// File permissions information
#[derive(Debug, Clone)]
pub struct FilePermissions {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}
