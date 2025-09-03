//! CLI argument parsing

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Path to the workflow YAML file (optional - will show selection menu if not provided)
    #[arg(value_name = "FILE")]
    pub file: Option<String>,

    /// List all available workflows
    #[arg(short, long)]
    pub list: bool,

    #[command(subcommand)]
    pub command: Option<Commands>
}

#[derive(Subcommand)]
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
        /// Optional Git URL to use instead of the configured one
        url:     Option<String>,
        /// Path to SSH private key for SSH authentication
        #[arg(long)]
        ssh_key: Option<String>
    }
}

#[derive(Subcommand)]
pub enum LangCommands {
    /// Set the current language
    Set {
        /// Language code (e.g., 'en', 'es')
        language: String
    },
    /// List available languages
    List,
    /// Show current language
    Current
}

#[derive(Subcommand)]
pub enum ResourceCommands {
    /// Set the resource URL for workflows
    Set {
        /// Git URL for workflows repository
        url: String
    },
    /// Show current resource URL
    Current
}
