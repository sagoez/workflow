use crate::domain::error::WorkflowError;

/// Port trait for user prompts (select, multi-select, text input)
pub trait UserPrompt: Send + Sync {
    /// Present a single-select prompt and return the chosen option
    fn select(&self, prompt: &str, options: Vec<String>, page_size: usize) -> Result<String, WorkflowError>;

    /// Present a multi-select prompt and return all chosen options
    fn multi_select(
        &self,
        prompt: &str,
        options: Vec<String>,
        page_size: usize,
        min: Option<usize>,
        max: Option<usize>
    ) -> Result<Vec<String>, WorkflowError>;

    /// Present a text input prompt and return the entered value
    fn text(&self, prompt: &str, default: Option<&str>) -> Result<String, WorkflowError>;
}
