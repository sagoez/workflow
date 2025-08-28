//! Spinner and progress indicator configurations

use indicatif::ProgressStyle;

/// Green spinner style for enum command execution
pub fn enum_spinner_style() -> ProgressStyle {
    ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .expect("Failed to create enum spinner style")
}

/// Blue spinner style for command execution
pub fn command_spinner_style() -> ProgressStyle {
    ProgressStyle::default_spinner()
        .template("{spinner:.blue} {msg}")
        .expect("Failed to create command spinner style")
}

/// Progress bar style for argument collection
pub fn progress_bar_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} {msg}")
        .expect("Failed to create progress bar style")
        .progress_chars("#>-")
}
