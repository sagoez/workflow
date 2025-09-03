//! Workflow YAML parsing functionality

use anyhow::Result;
use super::Workflow;

impl Workflow {
    /// Parse a workflow from YAML content.
    ///
    /// # Arguments
    /// * `yaml_content` - The YAML string content to parse
    ///
    /// # Returns
    /// * `Ok(Workflow)` - Successfully parsed workflow
    /// * `Err(serde_yaml::Error)` - YAML parsing error with details
    ///
    /// # Example
    /// ```rust
    /// use workflow::Workflow;
    ///
    /// let yaml = r#"
    /// name: "Test Workflow"
    /// command: "echo {{message}}"
    /// description: "A test workflow"
    /// arguments:
    ///   - name: message
    ///     description: "Message to echo"
    /// tags: []
    /// shells: ["bash"]
    /// "#;
    ///
    /// let workflow = Workflow::from_yaml(yaml).unwrap();
    /// assert_eq!(workflow.name, "Test Workflow");
    /// ```
    pub fn from_yaml(yaml_content: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml_content)
    }
}
