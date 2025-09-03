//! CLI command handlers

use anyhow::{Context, Result};

use crate::{config, i18n};
use super::{LangCommands, ResourceCommands};

/// Handle the init command - initialize configuration directories
pub async fn handle_init_command() -> Result<()> {
    println!("{}", i18n::t("init_initializing"));

    config::init_config_dirs().context("Failed to initialize configuration directories")?;

    let config_dir = config::get_config_dir()?;
    let workflows_dir = config::get_workflows_dir()?;
    let i18n_dir = config::get_i18n_dir()?;

    println!("{}", i18n::t("init_success"));
    println!("{}", i18n::t_params("init_config_dir", &[&config_dir.display().to_string()]));
    println!("{}", i18n::t_params("init_workflows_dir", &[&workflows_dir.display().to_string()]));
    println!("{}", i18n::t_params("init_i18n_dir", &[&i18n_dir.display().to_string()]));
    println!();
    println!("{}", i18n::t("init_instructions_header"));
    println!("{}", i18n::t_params("init_instructions_workflows", &[&workflows_dir.display().to_string()]));
    println!("{}", i18n::t_params("init_instructions_translations", &[&i18n_dir.display().to_string()]));
    println!("{}", i18n::t("init_instructions_language"));

    Ok(())
}

/// Handle language commands
pub async fn handle_lang_command(command: &LangCommands) -> Result<()> {
    match command {
        LangCommands::Set { language } => {
            // Validate that the language exists
            let available_languages =
                config::list_available_languages().context("Failed to list available languages")?;

            if !available_languages.contains(language) {
                anyhow::bail!(i18n::t_params("lang_unknown_language", &[language, &available_languages.join(", ")]));
            }

            config::set_language(language).context("Failed to set language")?;

            println!("{}", i18n::t_params("lang_set_success", &[language]));
        }
        LangCommands::List => {
            let languages = config::list_available_languages().context("Failed to list available languages")?;

            let current = config::get_current_language().unwrap_or_else(|_| "en".to_string());

            println!("{}", i18n::t("lang_available_header"));
            for lang in languages {
                let marker = if lang == current { i18n::t("lang_current_marker") } else { "".to_string() };
                println!("  • {}{}", lang, marker);
            }
        }
        LangCommands::Current => {
            let current = config::get_current_language().context("Failed to get current language")?;
            println!("{}", i18n::t_params("lang_current_language", &[&current]));
        }
    }

    Ok(())
}

/// Handle resource commands
pub async fn handle_resource_command(command: &ResourceCommands) -> Result<()> {
    match command {
        ResourceCommands::Set { url } => {
            config::set_resource_url(url).context("Failed to set resource URL")?;

            println!("{}", i18n::t_params("resource_set_success", &[url]));
            println!("{}", i18n::t("resource_set_tip"));
        }
        ResourceCommands::Current => {
            let current = config::get_current_resource_url().context("Failed to get current resource URL")?;

            match current {
                Some(url) => println!("{}", i18n::t_params("resource_current_url", &[&url])),
                None => println!("{}", i18n::t("resource_no_url"))
            }
        }
    }

    Ok(())
}

/// Handle sync command
pub async fn handle_sync_command(url: Option<&str>, ssh_key: Option<&str>) -> Result<()> {
    let workflows_dir = config::get_workflows_dir().context("Failed to get workflows directory")?;

    let resource_url = match url {
        Some(url) => url.to_string(),
        None => {
            let configured_url = config::get_current_resource_url().context("Failed to get current resource URL")?;

            match configured_url {
                Some(url) => url,
                None => {
                    anyhow::bail!("{}", i18n::t("sync_no_url_configured"));
                }
            }
        }
    };

    config::clone_workflows_from_git(&workflows_dir, &resource_url, ssh_key)?;

    println!("{}", i18n::t_params("sync_success", &[&resource_url]));
    Ok(())
}
