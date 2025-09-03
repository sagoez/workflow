//! Workflow execution functionality

use std::collections::HashMap;

use anyhow::{Context, Result};
use tera::{Context as TeraContext, Tera};

use super::{Workflow, resolver::resolve_workflow_arguments};
use crate::{i18n, ui, utils};

impl Workflow {
    /// Generate the command by resolving all arguments but don't execute it.
    ///
    /// This method is useful when you want to generate the command for manual execution,
    /// allowing users to copy/paste, modify, or pipe the command as needed.
    ///
    /// # Returns
    /// * `Ok(String)` - The fully resolved command string
    /// * `Err(anyhow::Error)` - Error during argument resolution or template rendering
    ///
    /// # Example
    /// ```rust,no_run
    /// use workflow::Workflow;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let yaml = r#"
    /// name: "Test Workflow"
    /// command: "echo {{message}}"
    /// description: "A test workflow"
    /// arguments:
    ///   - name: message
    ///     description: "Message to echo"
    ///     default_value: "Hello World"
    /// tags: []
    /// shells: ["bash"]
    /// "#;
    ///
    /// let workflow = Workflow::from_yaml(yaml)?;
    /// let command = workflow.generate_command().await?;
    /// println!("Generated command: {}", command);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_command(&self) -> Result<String> {
        ui::show_workflow_header(self);

        let argument_values = resolve_workflow_arguments(&self.arguments).await?;
        let final_command = self.render_command(&argument_values)?;

        // Copy the command to clipboard
        if let Err(e) = utils::copy_to_clipboard(&final_command) {
            eprintln!("⚠️  {}", e);
        }

        ui::show_final_command(&final_command);

        Ok(final_command)
    }

    /// Render the command template by substituting argument values.
    ///
    /// Uses the Tera templating engine to replace {{variable}} placeholders
    /// in the command string with the resolved argument values.
    ///
    /// # Arguments
    /// * `arguments` - Map of argument names to resolved values
    ///
    /// # Returns
    /// * `Ok(String)` - The rendered command ready for execution
    /// * `Err(anyhow::Error)` - Template rendering error (usually missing variables)
    ///
    /// # Example
    /// Command: `"echo {{message}} > {{file}}"`
    /// Arguments: `{"message": "Hello", "file": "output.txt"}`
    /// Result: `"echo Hello > output.txt"`
    fn render_command(&self, arguments: &HashMap<String, String>) -> Result<String> {
        let mut tera = Tera::default();
        let mut context = TeraContext::new();

        for (key, value) in arguments {
            context.insert(key, value);
        }

        tera.render_str(&self.command, &context).with_context(|| i18n::t("templates_render_failed"))
    }
}
