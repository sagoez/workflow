use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    sync::OnceLock
};

use crate::i18n::{
    Language,
    loader::{LanguageLoader, TextMap}
};

/// Global text manager for CLI
#[derive(Debug, Clone)]
pub struct TextManager {
    current_language: Language,
    cache:            HashMap<Language, TextMap>
}

/// Global text manager instance
static TEXT_MANAGER: OnceLock<TextManager> = OnceLock::new();

impl TextManager {
    /// Initialize the text manager with a config directory
    pub fn init(config_dir: Option<PathBuf>) -> &'static Self {
        TEXT_MANAGER.get_or_init(|| {
            let location = config_dir.unwrap_or_else(|| env::current_dir().unwrap_or_default());
            let loader = LanguageLoader::new(location.clone());

            let mut cache = HashMap::new();
            cache.insert(Language::English, loader.load(Language::English));
            cache.insert(Language::Spanish, loader.load(Language::Spanish));

            let current_language = Self::read_current_language_from_config(&location).unwrap_or(Language::English);

            Self { current_language, cache }
        })
    }

    /// Read the current language from the config file
    fn read_current_language_from_config(config_dir: &Path) -> Option<Language> {
        let lang_file = config_dir.join("language.txt");
        if lang_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&lang_file) {
                let lang_code = content.trim();
                Language::try_from(lang_code).ok()
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get text for a key in the current language
    pub fn get(&self, key: &str) -> String {
        self.cache
            .get(&self.current_language)
            .and_then(|texts| texts.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// Get text for a key in a specific language
    pub fn get_in_lang(&self, key: &str, lang: Language) -> String {
        self.cache.get(&lang).and_then(|texts| texts.get(key)).cloned().unwrap_or_else(|| key.to_string())
    }

    /// Set the current language
    pub fn set_language(&mut self, lang: Language) {
        self.current_language = lang;
    }

    /// Get the current language
    pub fn current_language(&self) -> Language {
        self.current_language
    }
}

/// Convenience function to get text
pub fn t(key: &str) -> String {
    TEXT_MANAGER.get().map(|tm| tm.get(key)).unwrap_or_else(|| key.to_string())
}

/// Convenience function to get text in specific language
pub fn t_lang(key: &str, lang: Language) -> String {
    TEXT_MANAGER.get().map(|tm| tm.get_in_lang(key, lang)).unwrap_or_else(|| key.to_string())
}

/// Convenience function to get parameterized text in current language
pub fn t_params(key: &str, params: &[&str]) -> String {
    let text = t(key);

    let mut result = text;
    for (i, param) in params.iter().enumerate() {
        let placeholder = format!("{{{}}}", i);
        result = result.replace(&placeholder, param);
    }

    result
}

/// Convenience function to get parameterized text in specific language
pub fn t_params_lang(key: &str, params: &[&str], lang: Language) -> String {
    let text = t_lang(key, lang);

    let mut result = text;
    for (i, param) in params.iter().enumerate() {
        let placeholder = format!("{{{}}}", i);
        result = result.replace(&placeholder, param);
    }

    result
}
