use crate::domain::error::WorkflowError;

/// A single entry in a select prompt with an optional hint shown next to the label.
#[derive(Debug, Clone)]
pub struct SelectOption {
    pub value: String,
    pub hint:  String
}

impl SelectOption {
    pub fn new(value: impl Into<String>, hint: impl Into<String>) -> Self {
        Self { value: value.into(), hint: hint.into() }
    }

    pub fn plain(value: impl Into<String>) -> Self {
        Self { value: value.into(), hint: String::new() }
    }
}

impl From<String> for SelectOption {
    fn from(value: String) -> Self {
        Self::plain(value)
    }
}

impl From<&str> for SelectOption {
    fn from(value: &str) -> Self {
        Self::plain(value.to_string())
    }
}

/// Port trait for user prompts (select, multi-select, text input, confirm)
pub trait UserPrompt: Send + Sync {
    /// Present a single-select prompt and return the chosen option value
    fn select(&self, prompt: &str, options: Vec<SelectOption>, page_size: usize) -> Result<String, WorkflowError>;

    /// Present a multi-select prompt and return all chosen option values
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

    /// Present a yes/no confirmation prompt
    fn confirm(&self, prompt: &str, default: bool) -> Result<bool, WorkflowError>;
}
