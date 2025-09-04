//! CLI argument definitions
//!
//! This module defines the command-line interface structure for the workflow application.

use clap::{Parser, Subcommand};

/// Main CLI application
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to the workflow YAML file (optional - will show selection menu if not provided)
    pub file: Option<String>,

    /// List all available workflows
    #[arg(short, long)]
    pub list: bool,

    /// Subcommands
    #[command(subcommand)]
    pub command: Option<Commands>
}

/// Available CLI commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize configuration directories and copy default files
    Init,
    /// Language management commands
    Lang {
        #[command(subcommand)]
        command: LangCommands
    },
    /// Resource management commands
    Resource {
        #[command(subcommand)]
        command: ResourceCommands
    },
    /// Sync workflows by cloning from Git repository
    Sync {
        /// Git repository URL to sync from
        url:     Option<String>,
        /// SSH key path for authentication
        #[arg(long)]
        ssh_key: Option<String>
    }
}

/// Language management subcommands
#[derive(Subcommand, Debug)]
pub enum LangCommands {
    /// Set the current language
    Set {
        /// Language code (e.g., 'en', 'es')
        language: String
    },
    /// Show current language
    Current,
    /// List available languages
    List
}

/// Resource management subcommands
#[derive(Subcommand, Debug)]
pub enum ResourceCommands {
    /// Set the resource URL
    Set {
        /// Resource URL to set
        url: String
    }
}
