use std::{collections::HashMap, path::PathBuf};

use serde_json::Value;

use crate::i18n::Language;

/// Type alias for translation text mappings (key -> translated text)
pub type TextMap = HashMap<String, String>;

/// Language loader for internationalization support
///
/// The `LanguageLoader` provides a two-tier loading strategy:
/// 1. **External files**: Attempts to load translations from JSON files in `{location}/i18n/`
/// 2. **Embedded fallback**: Falls back to compile-time embedded JSON files if external files fail
///
/// This approach ensures that:
/// - Users can override translations by providing external JSON files
/// - The application always has working translations (embedded as fallback)
/// - Installable binaries work without requiring external files
///
/// # Example
///
/// ```rust
/// use std::path::PathBuf;
///
/// use workflow::i18n::{Language, loader::LanguageLoader};
///
/// let config_dir = PathBuf::from("/path/to/config");
/// let loader = LanguageLoader::new(config_dir);
/// let translations = loader.load(Language::English);
/// ```
pub struct LanguageLoader {
    /// Base directory containing the `i18n/` subdirectory with translation files
    location: PathBuf
}

impl LanguageLoader {
    /// Creates a new language loader with the specified base directory
    ///
    /// The loader will look for translation files in `{location}/i18n/`
    ///
    /// # Arguments
    ///
    /// * `location` - Base directory path (translation files expected in `{location}/i18n/`)
    pub fn new(location: PathBuf) -> Self {
        Self { location }
    }

    /// Loads translations for the specified language
    ///
    /// This method implements a two-tier loading strategy:
    ///
    /// 1. **External file**: Attempts to read `{location}/i18n/{lang_code}.json`
    /// 2. **Embedded fallback**: Uses compile-time embedded JSON if external file fails
    ///
    /// # Arguments
    ///
    /// * `lang` - The language to load translations for
    ///
    /// # Returns
    ///
    /// A `TextMap` containing key-value pairs of translation keys and their translated text.
    /// Returns an empty HashMap if both external and embedded loading fail.
    ///
    /// # Example
    ///
    /// ```rust
    /// let loader = LanguageLoader::new(config_dir);
    /// let english_texts = loader.load(Language::English);
    /// let spanish_texts = loader.load(Language::Spanish);
    /// ```
    pub fn load(&self, lang: Language) -> TextMap {
        let i18n_dir = self.location.join("i18n");
        let config_path = i18n_dir.join(format!("{}.json", lang.code()));

        if let Ok(content) = std::fs::read_to_string(&config_path) {
            match serde_json::from_str::<HashMap<String, Value>>(&content) {
                Ok(json_map) => {
                    return json_map
                        .into_iter()
                        .filter_map(|(k, v)| if let Value::String(s) = v { Some((k, s)) } else { None })
                        .collect();
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", config_path.display(), e);
                }
            }
        }

        let embedded_content = match lang {
            Language::English => include_str!("../../config/i18n/en.json"),
            Language::Spanish => include_str!("../../config/i18n/es.json")
        };

        match serde_json::from_str::<HashMap<String, Value>>(embedded_content) {
            Ok(json_map) => json_map
                .into_iter()
                .filter_map(|(k, v)| if let Value::String(s) = v { Some((k, s)) } else { None })
                .collect(),
            Err(e) => {
                eprintln!("Warning: Failed to parse embedded translations for {}: {}", lang.code(), e);
                HashMap::new()
            }
        }
    }
}
