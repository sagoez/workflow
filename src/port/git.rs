//! Git repository management port traits
//!
//! This module defines the minimal git operations needed by the workflow system.

use std::path::Path;

use async_trait::async_trait;

use crate::domain::error::WorkflowError;

/// Configuration for cloning repositories
#[derive(Debug, Clone, Default)]
pub struct CloneOptions {
    /// SSH key path for authentication
    pub ssh_key: Option<String>,
    /// Branch to clone
    pub branch:  Option<String>
}

/// Commit information
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// Full commit ID
    pub id:           String,
    /// Short commit ID (first 8 characters)
    pub short_id:     String,
    /// Commit message
    pub message:      String,
    /// Author name
    pub author_name:  String,
    /// Author email
    pub author_email: String,
    /// Commit timestamp
    pub timestamp:    chrono::DateTime<chrono::Utc>
}

/// Git client trait for repository operations
#[async_trait]
pub trait GitClient: Send + Sync + 'static {
    /// Clone a repository to the specified destination, returns the commit ID
    async fn clone_repository(
        &self,
        url: &str,
        destination: &Path,
        options: &CloneOptions
    ) -> Result<String, WorkflowError>;

    /// Get commit information
    async fn get_commit_info(&self, repo_path: &Path, commit_id: Option<&str>) -> Result<CommitInfo, WorkflowError>;
}
