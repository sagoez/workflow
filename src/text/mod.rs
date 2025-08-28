//! # Text and Localization Module
//!
//! Provides internationalization support by loading text from YAML configuration files.
//! Each text is identified by an English key and can be localized to different languages.
//!
//! ## Usage
//! 
//! ```rust
//! use workflow::text::{get_text, set_language, Language};
//! 
//! // Get text in default language (English)
//! let msg = get_text("progress_collecting_arguments");
//! 
//! // Set language and get localized text
//! set_language(Language::Spanish);
//! let msg = get_text("progress_collecting_arguments");
//! ```
//!
//! ## Configuration
//!
//! Language files are stored in `config/i18n/` as YAML files:
//! - `config/i18n/en.yaml` - English (default)
//! - `config/i18n/es.yaml` - Spanish
//! - etc.

use std::collections::HashMap;
use std::sync::OnceLock;
use serde_yaml::Value;

pub mod spinners;

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Spanish,
    // French,
    // German,
    // Japanese,
}

impl Language {
    /// Get the file code for this language
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Spanish => "es",
        }
    }
    
    /// Parse language from code
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Language::English),
            "es" => Some(Language::Spanish),
            _ => None,
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::English
    }
}

/// Text mapping type
pub type TextMap = HashMap<String, String>;

/// Global text cache
static TEXT_CACHE: OnceLock<HashMap<Language, TextMap>> = OnceLock::new();

/// Load text mappings for a specific language
fn load_language_texts(lang: Language) -> TextMap {
    let config_path = format!("config/i18n/{}.yaml", lang.code());
    
    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            match serde_yaml::from_str::<HashMap<String, Value>>(&content) {
                Ok(yaml_map) => {
                    yaml_map.into_iter()
                        .filter_map(|(k, v)| {
                            if let Value::String(s) = v {
                                Some((k, s))
                            } else {
                                None
                            }
                        })
                        .collect()
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", config_path, e);
                    HashMap::new()
                }
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to load {}: {}", config_path, e);
            HashMap::new()
        }
    }
}

/// Initialize text cache
fn init_text_cache() -> HashMap<Language, TextMap> {
    let mut cache = HashMap::new();
    
    // Load all supported languages
    cache.insert(Language::English, load_language_texts(Language::English));
    cache.insert(Language::Spanish, load_language_texts(Language::Spanish));
    
    cache
}

/// Get the text cache, initializing if needed
fn get_text_cache() -> &'static HashMap<Language, TextMap> {
    TEXT_CACHE.get_or_init(init_text_cache)
}

/// Get text for a given key in the default language
pub fn get_text(key: &str) -> String {
    get_text_lang(key, current_language())
}

/// Get text for a given key in a specific language
pub fn get_text_lang(key: &str, lang: Language) -> String {
    let cache = get_text_cache();
    
    if let Some(text_map) = cache.get(&lang) {
        if let Some(text) = text_map.get(key) {
            return text.clone();
        }
    }
    
    // Fallback to English if key not found in target language
    if lang != Language::English {
        if let Some(en_map) = cache.get(&Language::English) {
            if let Some(text) = en_map.get(key) {
                return format!("[EN] {}", text);
            }
        }
    }
    
    format!("[MISSING: {}]", key)
}

/// Get formatted text with parameters
pub fn get_text_with_params(key: &str, params: &[&str]) -> String {
    get_text_with_params_lang(key, params, current_language())
}

/// Get formatted text with parameters in a specific language
pub fn get_text_with_params_lang(key: &str, params: &[&str], lang: Language) -> String {
    let template = get_text_lang(key, lang);
    
    // Simple parameter substitution: {0}, {1}, {2}, etc.
    let mut result = template;
    for (i, param) in params.iter().enumerate() {
        result = result.replace(&format!("{{{}}}", i), param);
    }
    
    result
}

/// Thread-local language setting for convenience
use std::cell::RefCell;
thread_local! {
    static CURRENT_LANGUAGE: RefCell<Language> = RefCell::new(Language::default());
}

/// Set the current language for this thread
pub fn set_language(lang: Language) {
    CURRENT_LANGUAGE.with(|l| *l.borrow_mut() = lang);
}

/// Get the current language for this thread
pub fn current_language() -> Language {
    CURRENT_LANGUAGE.with(|l| *l.borrow())
}

/// Convenience function to get text in current language
pub fn t(key: &str) -> String {
    get_text(key)
}

/// Convenience function to get parameterized text in current language
pub fn t_params(key: &str, params: &[&str]) -> String {
    get_text_with_params(key, params)
}