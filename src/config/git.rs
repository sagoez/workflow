use std::{fs, path::Path};

use anyhow::{Context, Result};
use git2::Repository;

use crate::i18n;

/// Clone workflows from a Git repository
pub fn clone_workflows_from_git(workflows_dir: &Path, resource_url: &str, ssh_key: Option<&str>) -> Result<()> {
    if workflows_dir.exists() {
        println!("{}", i18n::t("git_clearing_contents"));

        if let Ok(entries) = fs::read_dir(workflows_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Err(e) = fs::remove_file(&path) {
                        println!(
                            "{}",
                            i18n::t_params("git_warning_remove_file", &[&path.display().to_string(), &e.to_string()])
                        );
                    }
                } else if path.is_dir()
                    && let Err(e) = fs::remove_dir_all(&path)
                {
                    println!(
                        "{}",
                        i18n::t_params("git_warning_remove_dir", &[&path.display().to_string(), &e.to_string()])
                    );
                }
            }
            println!("{}", i18n::t("git_contents_cleared"));
        }
    } else {
        fs::create_dir_all(workflows_dir).with_context(|| {
            i18n::t_params("git_failed_to_create_workflows_dir", &[&workflows_dir.display().to_string()])
        })?;
    }

    println!("{}", i18n::t_params("git_cloning_from", &[resource_url]));

    // Create a temporary directory for cloning
    let temp_dir = workflows_dir.join("temp_clone");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }

    // Clone based on whether SSH key is provided
    let repo = if let Some(ssh_key_path) = ssh_key {
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

        builder
            .clone(resource_url, &temp_dir)
            .with_context(|| i18n::t_params("git_failed_to_clone", &[resource_url]))?
    } else {
        // Use default authentication (SSH agent, HTTPS, etc.)
        Repository::clone(resource_url, &temp_dir)
            .with_context(|| i18n::t_params("git_failed_to_clone", &[resource_url]))?
    };

    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    println!("{}", i18n::t_params("git_clone_success", &[&commit.id().to_string()]));

    // Move all files from the cloned repository to the workflows directory
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

            let target_path = workflows_dir.join(file_name);

            if path.is_file() {
                fs::copy(&path, &target_path)?;
            } else if path.is_dir() {
                if target_path.exists() {
                    fs::remove_dir_all(&target_path)?;
                }
                fs::rename(&path, &target_path)?;
            }
        }
    }

    // Clean up the temporary directory
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }

    Ok(())
}
