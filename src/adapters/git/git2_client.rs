//! Git2 implementation of git ports

use std::path::Path;

use async_trait::async_trait;
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository, build::RepoBuilder};

use crate::{
    ports::git::{CloneOptions, CommitInfo, GitClient, GitStatus, PullOptions},
    shared::WorkflowError
};

/// Git2 implementation of GitClient
pub struct Git2Client;

impl Git2Client {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Git2Client {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GitClient for Git2Client {
    async fn clone(&self, url: &str, destination: &Path, options: &CloneOptions) -> Result<(), WorkflowError> {
        // Clean up destination if it exists
        if destination.exists() {
            tokio::fs::remove_dir_all(destination).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
        }

        // Create parent directories
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
        }

        let mut builder = RepoBuilder::new();

        // Set up authentication if provided
        if options.ssh_key.is_some() || options.username.is_some() {
            let mut callbacks = RemoteCallbacks::new();

            let ssh_key = options.ssh_key.clone();
            let username = options.username.clone();
            let password = options.password.clone();

            callbacks.credentials(move |_url, username_from_url, _allowed_types| {
                if let Some(ssh_key_path) = &ssh_key {
                    let user = username.as_deref().or(username_from_url).unwrap_or("git");
                    Cred::ssh_key(user, None, Path::new(ssh_key_path), None)
                } else if let (Some(user), Some(pass)) = (&username, &password) {
                    Cred::userpass_plaintext(user, pass)
                } else {
                    Cred::default()
                }
            });

            let mut fetch_options = FetchOptions::new();
            fetch_options.remote_callbacks(callbacks);
            builder.fetch_options(fetch_options);
        }

        // Set branch if specified
        if let Some(branch) = &options.branch {
            builder.branch(branch);
        }

        // Set depth for shallow clone
        if let Some(_depth) = options.depth {
            // Note: git2 doesn't directly support shallow clones in the builder
            // This would require additional implementation using raw git commands
            // For now, we'll skip this feature
        }

        // Perform the clone
        let _repo = builder
            .clone(url, destination)
            .map_err(|e| WorkflowError::Network(format!("Failed to clone repository: {}", e)))?;

        Ok(())
    }

    async fn pull(&self, repo_path: &Path, options: &PullOptions) -> Result<(), WorkflowError> {
        let repo = Repository::open(repo_path)
            .map_err(|e| WorkflowError::Configuration(format!("Failed to open repository: {}", e)))?;

        let remote_name = options.remote.as_deref().unwrap_or("origin");
        let mut remote = repo
            .find_remote(remote_name)
            .map_err(|e| WorkflowError::Network(format!("Failed to find remote '{}': {}", remote_name, e)))?;

        // Set up authentication if provided
        let mut callbacks = RemoteCallbacks::new();

        if options.ssh_key.is_some() || options.username.is_some() {
            let ssh_key = options.ssh_key.clone();
            let username = options.username.clone();
            let password = options.password.clone();

            callbacks.credentials(move |_url, username_from_url, _allowed_types| {
                if let Some(ssh_key_path) = &ssh_key {
                    let user = username.as_deref().or(username_from_url).unwrap_or("git");
                    Cred::ssh_key(user, None, Path::new(ssh_key_path), None)
                } else if let (Some(user), Some(pass)) = (&username, &password) {
                    Cred::userpass_plaintext(user, pass)
                } else {
                    Cred::default()
                }
            });
        }

        // Fetch from remote
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let refspecs =
            remote.fetch_refspecs().map_err(|e| WorkflowError::Network(format!("Failed to get refspecs: {}", e)))?;

        let refspecs_vec: Vec<&str> = refspecs.iter().flatten().collect();
        remote
            .fetch(&refspecs_vec, Some(&mut fetch_options), None)
            .map_err(|e| WorkflowError::Network(format!("Failed to fetch: {}", e)))?;

        // Merge or fast-forward
        let fetch_head = repo
            .find_reference("FETCH_HEAD")
            .map_err(|e| WorkflowError::Network(format!("Failed to find FETCH_HEAD: {}", e)))?;

        let fetch_commit = repo
            .reference_to_annotated_commit(&fetch_head)
            .map_err(|e| WorkflowError::Network(format!("Failed to get fetch commit: {}", e)))?;

        // Perform merge analysis
        let analysis = repo
            .merge_analysis(&[&fetch_commit])
            .map_err(|e| WorkflowError::Network(format!("Failed to analyze merge: {}", e)))?;

        if analysis.0.is_fast_forward() {
            // Fast-forward merge
            let refname = format!("refs/heads/{}", options.branch.as_deref().unwrap_or("main"));

            if let Ok(mut reference) = repo.find_reference(&refname) {
                reference
                    .set_target(fetch_commit.id(), "Fast-forward")
                    .map_err(|e| WorkflowError::Network(format!("Failed to fast-forward: {}", e)))?;

                repo.set_head(&refname).map_err(|e| WorkflowError::Network(format!("Failed to set HEAD: {}", e)))?;

                repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                    .map_err(|e| WorkflowError::Network(format!("Failed to checkout: {}", e)))?;
            }
        } else if analysis.0.is_normal() {
            // Normal merge would be more complex - for now, just error
            return Err(WorkflowError::Network("Normal merge not implemented - please resolve manually".to_string()));
        }

        Ok(())
    }

    async fn status(&self, repo_path: &Path) -> Result<GitStatus, WorkflowError> {
        let repo = Repository::open(repo_path)
            .map_err(|e| WorkflowError::Configuration(format!("Failed to open repository: {}", e)))?;

        let statuses =
            repo.statuses(None).map_err(|e| WorkflowError::Network(format!("Failed to get status: {}", e)))?;

        let mut modified_files = Vec::new();
        let mut untracked_files = Vec::new();
        let mut staged_files = Vec::new();

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let status = entry.status();

                if status.is_wt_modified() || status.is_wt_deleted() || status.is_wt_renamed() {
                    modified_files.push(path.to_string());
                }

                if status.is_wt_new() {
                    untracked_files.push(path.to_string());
                }

                if status.is_index_modified() || status.is_index_new() || status.is_index_deleted() {
                    staged_files.push(path.to_string());
                }
            }
        }

        let head = repo.head().map_err(|e| WorkflowError::Network(format!("Failed to get HEAD: {}", e)))?;

        let current_branch = head.shorthand().unwrap_or("unknown").to_string();

        let is_clean = modified_files.is_empty() && untracked_files.is_empty() && staged_files.is_empty();

        Ok(GitStatus {
            is_clean,
            modified_files,
            untracked_files,
            staged_files,
            current_branch,
            ahead_by: 0,  // Would need additional implementation
            behind_by: 0  // Would need additional implementation
        })
    }

    async fn get_commit_info(&self, repo_path: &Path, commit_id: Option<&str>) -> Result<CommitInfo, WorkflowError> {
        let repo = Repository::open(repo_path)
            .map_err(|e| WorkflowError::Configuration(format!("Failed to open repository: {}", e)))?;

        let commit = if let Some(id) = commit_id {
            let oid =
                git2::Oid::from_str(id).map_err(|e| WorkflowError::Validation(format!("Invalid commit ID: {}", e)))?;
            repo.find_commit(oid).map_err(|e| WorkflowError::Network(format!("Failed to find commit: {}", e)))?
        } else {
            let head = repo.head().map_err(|e| WorkflowError::Network(format!("Failed to get HEAD: {}", e)))?;
            head.peel_to_commit().map_err(|e| WorkflowError::Network(format!("Failed to get HEAD commit: {}", e)))?
        };

        let author = commit.author();

        Ok(CommitInfo {
            id:           commit.id().to_string(),
            short_id:     commit.id().to_string()[..8].to_string(),
            message:      commit.message().unwrap_or("").to_string(),
            author_name:  author.name().unwrap_or("").to_string(),
            author_email: author.email().unwrap_or("").to_string(),
            timestamp:    chrono::DateTime::from_timestamp(author.when().seconds(), 0)
                .unwrap_or_default()
                .with_timezone(&chrono::Utc)
        })
    }

    async fn list_branches(&self, repo_path: &Path) -> Result<Vec<String>, WorkflowError> {
        let repo = Repository::open(repo_path)
            .map_err(|e| WorkflowError::Configuration(format!("Failed to open repository: {}", e)))?;

        let branches = repo
            .branches(Some(git2::BranchType::Local))
            .map_err(|e| WorkflowError::Network(format!("Failed to list branches: {}", e)))?;

        let mut branch_names = Vec::new();

        for branch in branches {
            let (branch, _) = branch.map_err(|e| WorkflowError::Network(format!("Failed to get branch: {}", e)))?;

            if let Some(name) =
                branch.name().map_err(|e| WorkflowError::Network(format!("Failed to get branch name: {}", e)))?
            {
                branch_names.push(name.to_string());
            }
        }

        Ok(branch_names)
    }

    async fn is_repository(&self, path: &Path) -> Result<bool, WorkflowError> {
        Ok(Repository::open(path).is_ok())
    }

    async fn get_remote_url(&self, repo_path: &Path, remote_name: &str) -> Result<String, WorkflowError> {
        let repo = Repository::open(repo_path)
            .map_err(|e| WorkflowError::Configuration(format!("Failed to open repository: {}", e)))?;

        let remote = repo
            .find_remote(remote_name)
            .map_err(|e| WorkflowError::Network(format!("Failed to find remote '{}': {}", remote_name, e)))?;

        let url = remote.url().ok_or_else(|| WorkflowError::Network(format!("Remote '{}' has no URL", remote_name)))?;

        Ok(url.to_string())
    }
}
