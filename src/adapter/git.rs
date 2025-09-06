//! Git2 implementation of git ports

use std::{fs, path::Path};

use anyhow::Context;
use async_trait::async_trait;
use git2::Repository;

use crate::{
    domain::error::WorkflowError,
    port::git::{CloneOptions, CommitInfo, GitClient},
    t, t_params
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
    async fn clone_repository(
        &self,
        url: &str,
        destination: &Path,
        options: &CloneOptions
    ) -> Result<String, WorkflowError> {
        if destination.exists() {
            println!("{}", t!("git_clearing_contents"));

            if let Ok(entries) = fs::read_dir(destination) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Err(e) = fs::remove_file(&path) {
                            println!(
                                "{}",
                                t_params!("git_warning_remove_file", &[&path.display().to_string(), &e.to_string()])
                            );
                        }
                    } else if path.is_dir()
                        && let Err(e) = fs::remove_dir_all(&path)
                    {
                        println!(
                            "{}",
                            t_params!("git_warning_remove_dir", &[&path.display().to_string(), &e.to_string()])
                        );
                    }
                }
                println!("{}", t!("git_contents_cleared"));
            }
        } else {
            fs::create_dir_all(destination)
                .with_context(|| t_params!("git_failed_to_create_workflows_dir", &[&destination.display().to_string()]))
                .map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
        }

        println!("{}", t_params!("git_cloning_from", &[url]));

        // Create a temporary directory for cloning
        let temp_dir = destination.join("temp_clone");
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
        }

        // Clone based on whether SSH key is provided
        let repo = if let Some(ssh_key_path) = &options.ssh_key {
            // Use SSH key for authentication
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.credentials(move |_user, _user_from_url, _cred| {
                let path = std::path::Path::new(ssh_key_path);
                git2::Cred::ssh_key("git", None, path, None)
            });

            let mut fetch_opts = git2::FetchOptions::new();
            fetch_opts.remote_callbacks(callbacks);

            let mut builder = git2::build::RepoBuilder::new();
            builder.fetch_options(fetch_opts);

            // Set branch if specified
            if let Some(branch) = &options.branch {
                builder.branch(branch);
            }

            builder.clone(url, &temp_dir).map_err(|e| {
                WorkflowError::Network(t_params!("git_failed_to_clone_with_ssh_key", &[url, &e.to_string()]))
            })?
        } else {
            // Use default authentication (SSH agent, HTTPS, etc.)
            Repository::clone(url, &temp_dir).map_err(|e| {
                WorkflowError::Network(t_params!(
                    "git_failed_to_clone_with_default_authentication",
                    &[url, &e.to_string()]
                ))
            })?
        };

        let head = repo.head().map_err(|e| WorkflowError::Network(e.to_string()))?;
        let commit = head.peel_to_commit().map_err(|e| WorkflowError::Network(e.to_string()))?;
        let commit_id = commit.id().to_string();

        println!("{}", t_params!("git_clone_success", &[&commit_id]));

        // Move all files from the cloned repository to the destination directory
        // Skip the .git directory and any other hidden files
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().unwrap_or_default();

                // Skip .git directory and other hidden files
                if let Some(name) = file_name.to_str()
                    && name.starts_with('.')
                {
                    continue;
                }

                let target_path = destination.join(file_name);

                if path.is_file() {
                    fs::copy(&path, &target_path).map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
                } else if path.is_dir() {
                    if target_path.exists() {
                        fs::remove_dir_all(&target_path).map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
                    }
                    fs::rename(&path, &target_path).map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
                }
            }
        }

        // Clean up the temporary directory
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).map_err(|e| WorkflowError::FileSystem(e.to_string()))?;
        }

        Ok(commit_id)
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
}
