use crate::domain::error::WorkflowError;

pub mod display;
pub mod loader;
pub mod macros;

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Spanish /* French,
             * German,
             * Chinese, */
}

impl Language {
    /// Get the file code for this language
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Spanish => "es"
        }
    }
}

impl TryFrom<&str> for Language {
    type Error = WorkflowError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "en" => Ok(Language::English),
            "es" => Ok(Language::Spanish),
            _ => {
                use crate::t_params;
                Err(WorkflowError::UnsupportedLanguage(t_params!("error_unsupported_language", &[value])))
            }
        }
    }
}
