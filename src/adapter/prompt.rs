use crate::{domain::error::WorkflowError, port::prompt::UserPrompt};

/// Real implementation wrapping the `inquire` crate
pub struct InquirePrompt;

impl InquirePrompt {
    pub fn new() -> Self {
        Self
    }
}

impl UserPrompt for InquirePrompt {
    fn select(&self, prompt: &str, options: Vec<String>, page_size: usize) -> Result<String, WorkflowError> {
        inquire::Select::new(prompt, options)
            .with_page_size(page_size)
            .prompt()
            .map_err(|e| WorkflowError::UserInteraction(e.to_string()))
    }

    fn multi_select(
        &self,
        prompt: &str,
        options: Vec<String>,
        page_size: usize,
        min: Option<usize>,
        max: Option<usize>
    ) -> Result<Vec<String>, WorkflowError> {
        let mut ms = inquire::MultiSelect::new(prompt, options).with_page_size(page_size);

        if let Some(min_val) = min {
            ms = ms.with_validator(move |selections: &[inquire::list_option::ListOption<&String>]| {
                if selections.len() < min_val {
                    Ok(inquire::validator::Validation::Invalid(
                        format!("Select at least {} item(s)", min_val).into(),
                    ))
                } else {
                    Ok(inquire::validator::Validation::Valid)
                }
            });
        }

        if let Some(max_val) = max {
            ms = ms.with_validator(move |selections: &[inquire::list_option::ListOption<&String>]| {
                if selections.len() > max_val {
                    Ok(inquire::validator::Validation::Invalid(
                        format!("Select at most {} item(s)", max_val).into(),
                    ))
                } else {
                    Ok(inquire::validator::Validation::Valid)
                }
            });
        }

        ms.prompt().map_err(|e| WorkflowError::UserInteraction(e.to_string()))
    }

    fn text(&self, prompt: &str, default: Option<&str>) -> Result<String, WorkflowError> {
        let mut text_input = inquire::Text::new(prompt);
        if let Some(d) = default {
            text_input = text_input.with_default(d);
        }
        text_input.prompt().map_err(|e| WorkflowError::UserInteraction(e.to_string()))
    }
}

#[cfg(test)]
pub mod mock {
    use std::sync::Mutex;

    use super::*;

    /// Represents a scripted response for MockPrompt
    pub enum MockPromptResponse {
        Select(String),
        MultiSelect(Vec<String>),
        Text(String),
        Error(WorkflowError)
    }

    /// Mock implementation that returns scripted responses in order
    pub struct MockPrompt {
        responses: Mutex<Vec<MockPromptResponse>>
    }

    impl MockPrompt {
        pub fn new(responses: Vec<MockPromptResponse>) -> Self {
            Self { responses: Mutex::new(responses) }
        }
    }

    impl UserPrompt for MockPrompt {
        fn select(&self, _prompt: &str, _options: Vec<String>, _page_size: usize) -> Result<String, WorkflowError> {
            let mut responses = self.responses.lock().unwrap();
            match responses.remove(0) {
                MockPromptResponse::Select(value) => Ok(value),
                MockPromptResponse::Error(e) => Err(e),
                _ => panic!("MockPrompt: expected Select response")
            }
        }

        fn multi_select(
            &self,
            _prompt: &str,
            _options: Vec<String>,
            _page_size: usize,
            _min: Option<usize>,
            _max: Option<usize>
        ) -> Result<Vec<String>, WorkflowError> {
            let mut responses = self.responses.lock().unwrap();
            match responses.remove(0) {
                MockPromptResponse::MultiSelect(values) => Ok(values),
                MockPromptResponse::Error(e) => Err(e),
                _ => panic!("MockPrompt: expected MultiSelect response")
            }
        }

        fn text(&self, _prompt: &str, _default: Option<&str>) -> Result<String, WorkflowError> {
            let mut responses = self.responses.lock().unwrap();
            match responses.remove(0) {
                MockPromptResponse::Text(value) => Ok(value),
                MockPromptResponse::Error(e) => Err(e),
                _ => panic!("MockPrompt: expected Text response")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{mock::*, *};

    #[test]
    fn mock_prompt_select_returns_scripted_value() {
        let mock = MockPrompt::new(vec![MockPromptResponse::Select("option-b".to_string())]);
        let result = mock.select("Pick one", vec!["option-a".into(), "option-b".into()], 10);
        assert_eq!(result.unwrap(), "option-b");
    }

    #[test]
    fn mock_prompt_multi_select_returns_scripted_values() {
        let mock = MockPrompt::new(vec![MockPromptResponse::MultiSelect(vec![
            "a".to_string(),
            "c".to_string(),
        ])]);
        let result = mock.multi_select("Pick many", vec!["a".into(), "b".into(), "c".into()], 10, None, None);
        assert_eq!(result.unwrap(), vec!["a", "c"]);
    }

    #[test]
    fn mock_prompt_text_returns_scripted_value() {
        let mock = MockPrompt::new(vec![MockPromptResponse::Text("hello".to_string())]);
        let result = mock.text("Enter text", None);
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn mock_prompt_returns_error() {
        let mock = MockPrompt::new(vec![MockPromptResponse::Error(
            WorkflowError::UserInteraction("cancelled".to_string()),
        )]);
        let result = mock.select("Pick", vec!["a".into()], 10);
        assert!(result.is_err());
    }

    #[test]
    fn mock_prompt_sequences_multiple_responses() {
        let mock = MockPrompt::new(vec![
            MockPromptResponse::Select("first".to_string()),
            MockPromptResponse::Text("second".to_string()),
        ]);
        assert_eq!(mock.select("p", vec!["first".into()], 10).unwrap(), "first");
        assert_eq!(mock.text("p", None).unwrap(), "second");
    }
}
