use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    AppContext,
    domain::{
        command::{GetCurrentLanguageCommand, ListLanguagesCommand, SetLanguageCommand},
        engine::EngineContext,
        error::WorkflowError,
        event::{LanguageSetEvent, WorkflowEvent},
        state::WorkflowState
    },
    i18n::Language,
    port::command::Command,
    t, t_params
};

/// Validate a language code string and return the corresponding Language enum.
pub fn validate_language(code: &str) -> Result<Language, WorkflowError> {
    Language::try_from(code)
}

/// Return the list of available language codes.
pub fn available_languages() -> Vec<String> {
    vec![Language::English.code().to_string(), Language::Spanish.code().to_string()]
}

#[async_trait]
impl Command for SetLanguageCommand {
    type Error = WorkflowError;
    type LoadedData = ();

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        Ok(())
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        validate_language(&self.language)?;
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        let event = LanguageSetEvent {
            event_id:  Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            language:  self.language.clone()
        };

        Ok(vec![WorkflowEvent::LanguageSet(event)])
    }

    async fn effect(
        &self,
        _loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        current_state: &WorkflowState,
        _context: &EngineContext,
        app_context: &AppContext
    ) -> Result<(), Self::Error> {
        match current_state {
            WorkflowState::LanguageSet(state) => {
                let language = Language::try_from(state.language.as_str())?;
                app_context.config.set_current_language(language)?;
                println!("{}", t_params!("lang_set_success", &[&state.language]));
            }
            _ => {
                return Err(WorkflowError::Validation("Invalid state for language set".to_string()));
            }
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "set-language"
    }

    fn description(&self) -> &'static str {
        "Sets the current language for the application"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        true
    }
}

#[async_trait]
impl Command for GetCurrentLanguageCommand {
    type Error = WorkflowError;
    type LoadedData = String;

    async fn load(
        &self,
        _context: &EngineContext,
        app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        let current_language = app_context.config.get_current_language()?;
        Ok(current_language.code().to_string())
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        Ok(vec![])
    }

    async fn effect(
        &self,
        loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        _current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        println!("{}", t_params!("lang_current", &[&loaded_data]));
        Ok(())
    }

    fn name(&self) -> &'static str {
        "get-current-language"
    }

    fn description(&self) -> &'static str {
        "Gets the current language setting"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        false
    }
}

#[async_trait]
impl Command for ListLanguagesCommand {
    type Error = WorkflowError;
    type LoadedData = Vec<String>;

    async fn load(
        &self,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Self::LoadedData, Self::Error> {
        Ok(available_languages())
    }

    fn validate(&self, _loaded_data: &Self::LoadedData) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn emit(
        &self,
        _loaded_data: &Self::LoadedData,
        _context: &EngineContext,
        _app_context: &AppContext,
        _current_state: &WorkflowState
    ) -> Result<Vec<WorkflowEvent>, Self::Error> {
        Ok(vec![])
    }

    async fn effect(
        &self,
        loaded_data: &Self::LoadedData,
        _previous_state: &WorkflowState,
        _current_state: &WorkflowState,
        _context: &EngineContext,
        _app_context: &AppContext
    ) -> Result<(), Self::Error> {
        println!("{}", t!("lang_available_languages"));
        println!();
        for language in loaded_data {
            println!("  - {}", language);
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "list-languages"
    }

    fn description(&self) -> &'static str {
        "Lists all available languages"
    }

    fn is_interactive(&self) -> bool {
        false
    }

    fn is_mutating(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_valid_language() {
        let lang = validate_language("en").unwrap();
        assert!(matches!(lang, Language::English));

        let lang = validate_language("es").unwrap();
        assert!(matches!(lang, Language::Spanish));
    }

    #[test]
    fn validate_invalid_language_returns_error() {
        let result = validate_language("xx");
        assert!(result.is_err());
    }

    #[test]
    fn available_languages_returns_en_and_es() {
        let langs = available_languages();
        assert_eq!(langs, vec!["en", "es"]);
    }
}
