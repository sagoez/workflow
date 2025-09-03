//! Progress indicators and spinner styles

use indicatif::{ProgressBar, ProgressStyle};

/// Green spinner style for enum command execution
pub fn enum_spinner_style() -> ProgressStyle {
    ProgressStyle::default_spinner().template("{spinner:.green} {msg}").expect("Failed to create enum spinner style")
}

/// Create a spinner for enum argument resolution
pub fn create_enum_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(enum_spinner_style());
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));
    spinner
}
