use std::collections::HashMap;

use serde_yaml::Value;

use super::{Language, TextMap};
use crate::config;

/// Load text mappings for a specific language
pub fn load_language_texts(lang: Language) -> TextMap {
    if let Ok(i18n_dir) = config::get_i18n_dir() {
        let config_path = i18n_dir.join(format!("{}.yaml", lang.code()));

        if let Ok(content) = std::fs::read_to_string(&config_path) {
            match serde_yaml::from_str::<HashMap<String, Value>>(&content) {
                Ok(yaml_map) => {
                    return yaml_map
                        .into_iter()
                        .filter_map(|(k, v)| if let Value::String(s) = v { Some((k, s)) } else { None })
                        .collect();
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", config_path.display(), e);
                }
            }
        }
    }

    let embedded_content = match lang {
        Language::English => include_str!("../../config/i18n/en.yaml"),
        Language::Spanish => include_str!("../../config/i18n/es.yaml")
    };

    match serde_yaml::from_str::<HashMap<String, Value>>(embedded_content) {
        Ok(yaml_map) => yaml_map
            .into_iter()
            .filter_map(|(k, v)| if let Value::String(s) = v { Some((k, s)) } else { None })
            .collect(),
        Err(e) => {
            eprintln!("Warning: Failed to parse embedded translations for {}: {}", lang.code(), e);
            HashMap::new()
        }
    }
}
