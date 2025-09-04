//! Git ports - interfaces for Git operations

use async_trait::async_trait;
use std::path::Path;

use crate::shared::WorkflowError;

/// Port for Git operations
#[async_trait]
pub trait GitClient: Send + Sync {
    /// Clone a repository to a local directory
    async fn clone(&self, url: &str, destination: &Path, options: &CloneOptions) -> Result<(), WorkflowError>;
    
    /// Pull latest changes from remote
    async fn pull(&self, repo_path: &Path, options: &PullOptions) -> Result<(), WorkflowError>;
    
    /// Get repository status
    async fn status(&self, repo_path: &Path) -> Result<GitStatus, WorkflowError>;
    
    /// Get commit information
    async fn get_commit_info(&self, repo_path: &Path, commit_id: Option<&str>) -> Result<CommitInfo, WorkflowError>;
    
    /// List branches
    async fn list_branches(&self, repo_path: &Path) -> Result<Vec<String>, WorkflowError>;
    
    /// Check if directory is a git repository
    async fn is_repository(&self, path: &Path) -> Result<bool, WorkflowError>;
    
    /// Get remote URL
    async fn get_remote_url(&self, repo_path: &Path, remote_name: &str) -> Result<String, WorkflowError>;
}

/// Options for cloning repositories
#[derive(Debug, Clone, Default)]
pub struct CloneOptions {
    /// SSH private key path for authentication
    pub ssh_key: Option<String>,
    /// Username for authentication
    pub username: Option<String>,
    /// Password for authentication  
    pub password: Option<String>,
    /// Specific branch to clone
    pub branch: Option<String>,
    /// Shallow clone depth
    pub depth: Option<u32>,
}

/// Options for pulling changes
#[derive(Debug, Clone, Default)]
pub struct PullOptions {
    /// SSH private key path for authentication
    pub ssh_key: Option<String>,
    /// Username for authentication
    pub username: Option<String>,
    /// Password for authentication
    pub password: Option<String>,
    /// Remote name (default: origin)
    pub remote: Option<String>,
    /// Branch name (default: current branch)
    pub branch: Option<String>,
}

/// Git repository status
#[derive(Debug, Clone)]
pub struct GitStatus {
    pub is_clean: bool,
    pub modified_files: Vec<String>,
    pub untracked_files: Vec<String>,
    pub staged_files: Vec<String>,
    pub current_branch: String,
    pub ahead_by: u32,
    pub behind_by: u32,
}

/// Git commit information
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
