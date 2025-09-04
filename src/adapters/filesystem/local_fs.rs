//! Local filesystem implementation of filesystem ports

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::fs;

use crate::{
    ports::filesystem::{FileMetadata, FilePermissions, FileSystem},
    shared::WorkflowError
};

/// Local filesystem implementation
pub struct LocalFileSystem;

impl LocalFileSystem {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileSystem for LocalFileSystem {
    async fn read_to_string(&self, path: &Path) -> Result<String, WorkflowError> {
        fs::read_to_string(path).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))
    }

    async fn write_string(&self, path: &Path, contents: &str) -> Result<(), WorkflowError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
        }

        fs::write(path, contents).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))
    }

    async fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, WorkflowError> {
        fs::read(path).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))
    }

    async fn write_bytes(&self, path: &Path, contents: &[u8]) -> Result<(), WorkflowError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
        }

        fs::write(path, contents).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))
    }

    async fn exists(&self, path: &Path) -> Result<bool, WorkflowError> {
        Ok(path.exists())
    }

    async fn is_file(&self, path: &Path) -> Result<bool, WorkflowError> {
        Ok(path.is_file())
    }

    async fn is_dir(&self, path: &Path) -> Result<bool, WorkflowError> {
        Ok(path.is_dir())
    }

    async fn create_dir_all(&self, path: &Path) -> Result<(), WorkflowError> {
        fs::create_dir_all(path).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))
    }

    async fn remove_file(&self, path: &Path) -> Result<(), WorkflowError> {
        fs::remove_file(path).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))
    }

    async fn remove_dir_all(&self, path: &Path) -> Result<(), WorkflowError> {
        fs::remove_dir_all(path).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))
    }

    async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, WorkflowError> {
        let mut entries = Vec::new();
        let mut dir = fs::read_dir(path).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;

        while let Some(entry) = dir.next_entry().await.map_err(|e| WorkflowError::FileSystem(e.to_string()))? {
            entries.push(entry.path());
        }

        Ok(entries)
    }

    async fn copy_file(&self, from: &Path, to: &Path) -> Result<(), WorkflowError> {
        // Ensure destination parent directory exists
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
        }

        fs::copy(from, to).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;

        Ok(())
    }

    async fn move_file(&self, from: &Path, to: &Path) -> Result<(), WorkflowError> {
        // Ensure destination parent directory exists
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
        }

        fs::rename(from, to).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))
    }

    async fn metadata(&self, path: &Path) -> Result<FileMetadata, WorkflowError> {
        let metadata = fs::metadata(path).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;

        let permissions = FilePermissions {
            readable:   true, // On Unix, we'd check metadata.permissions()
            writable:   !metadata.permissions().readonly(),
            executable: false // Would need platform-specific code
        };

        Ok(FileMetadata {
            size: metadata.len(),
            is_file: metadata.is_file(),
            is_dir: metadata.is_dir(),
            modified: metadata.modified().ok(),
            created: metadata.created().ok(),
            permissions
        })
    }

    async fn find_files(&self, dir: &Path, pattern: &str) -> Result<Vec<PathBuf>, WorkflowError> {
        let mut found_files = Vec::new();

        if !dir.exists() || !dir.is_dir() {
            return Ok(found_files);
        }

        let mut stack = vec![dir.to_path_buf()];

        while let Some(current_dir) = stack.pop() {
            let mut entries = fs::read_dir(&current_dir).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| WorkflowError::FileSystem(e.to_string()))? {
                let path = entry.path();

                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    // Simple pattern matching - could be enhanced with regex or glob
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if pattern == "*" || file_name.contains(pattern) || file_name.ends_with(pattern) {
                            found_files.push(path);
                        }
                    }
                }
            }
        }

        Ok(found_files)
    }
}
