use anyhow::Result;
use clipboard::{ClipboardContext, ClipboardProvider};

use crate::i18n;

/// Copy text to clipboard with user feedback
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    if let Ok(mut clipboard) = ClipboardContext::new() {
        if let Err(e) = clipboard.set_contents(text.to_string()) {
            eprintln!("⚠️  {}", i18n::t_params("clipboard_copy_failed", &[&e.to_string()]));
            anyhow::bail!("Failed to copy to clipboard: {}", e);
        } else {
            println!("{}", i18n::t("command_copied_to_clipboard"));
        }
    } else {
        eprintln!("⚠️  {}", i18n::t("clipboard_access_failed"));
        anyhow::bail!("Failed to access clipboard");
    }
    Ok(())
}
