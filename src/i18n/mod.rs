mod loader;

use std::{collections::HashMap, sync::OnceLock};

pub use loader::*;

use crate::config;

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    English,
    Spanish /* French,
             * German,
             * Japanese, */
}

impl Language {
    /// Get the file code for this language
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Spanish => "es"
        }
    }

    /// Parse language from code
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Language::English),
            "es" => Some(Language::Spanish),
            _ => None
        }
    }
}

/// Text mapping type
pub type TextMap = HashMap<String, String>;

/// Global text cache
static TEXT_CACHE: OnceLock<HashMap<Language, TextMap>> = OnceLock::new();

/// Get the text cache, initializing if needed
fn get_text_cache() -> &'static HashMap<Language, TextMap> {
    TEXT_CACHE.get_or_init(|| {
        let mut cache = HashMap::new();
        cache.insert(Language::English, load_language_texts(Language::English));
        cache.insert(Language::Spanish, load_language_texts(Language::Spanish));
        cache
    })
}

/// Get text for a given key in the default language
pub fn get_text(key: &str) -> String {
    get_text_lang(key, current_language())
}

/// Get text for a given key in a specific language
pub fn get_text_lang(key: &str, lang: Language) -> String {
    let cache = get_text_cache();

    if let Some(text_map) = cache.get(&lang)
        && let Some(text) = text_map.get(key)
    {
        return text.clone();
    }

    if lang != Language::English
        && let Some(en_map) = cache.get(&Language::English)
        && let Some(text) = en_map.get(key)
    {
        return format!("[EN] {}", text);
    }

    format!("[MISSING: {}]", key)
}

/// Get formatted text with parameters
fn get_text_with_params(key: &str, params: &[&str]) -> String {
    get_text_with_params_lang(key, params, current_language())
}

/// Get formatted text with parameters in a specific language
fn get_text_with_params_lang(key: &str, params: &[&str], lang: Language) -> String {
    let template = get_text_lang(key, lang);

    let mut result = template;
    for (i, param) in params.iter().enumerate() {
        result = result.replace(&format!("{{{}}}", i), param);
    }

    result
}

/// Get the current language from configuration
pub fn current_language() -> Language {
    config::get_current_language().ok().and_then(|lang_code| Language::from_code(&lang_code)).unwrap_or_default()
}

/// Convenience function to get text in current language
pub fn t(key: &str) -> String {
    get_text(key)
}

/// Convenience function to get parameterized text in current language
pub fn t_params(key: &str, params: &[&str]) -> String {
    get_text_with_params(key, params)
}
