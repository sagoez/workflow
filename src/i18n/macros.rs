/// Compile-time translation key validation macro
///
/// This macro validates that translation keys exist in the embedded JSON files
/// at compile time, preventing runtime errors from typos or missing keys.
///
/// Usage:
/// ```
/// let text = t!("welcome_message");
/// let text = t!("error_file_not_found", Language::Spanish);
/// ```
#[macro_export]
macro_rules! t {
    ($key:literal) => {{
        const EN_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/i18n/en.json"));
        const ES_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/i18n/es.json"));

        $crate::validate_key!(EN_JSON, ES_JSON, $key);

        $crate::i18n::display::t($key)
    }};

    ($key:literal, $lang:expr) => {{
        const EN_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/i18n/en.json"));
        const ES_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/i18n/es.json"));

        $crate::validate_key!(EN_JSON, ES_JSON, $key);

        $crate::i18n::display::t_lang($key, $lang)
    }};
}

/// Compile-time validated parameterized translation macro
#[macro_export]
macro_rules! t_params {
    ($key:literal, $params:expr) => {{
        const EN_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/i18n/en.json"));
        const ES_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/i18n/es.json"));

        $crate::validate_key!(EN_JSON, ES_JSON, $key);

        $crate::i18n::display::t_params($key, $params)
    }};

    ($key:literal, $params:expr, $lang:expr) => {{
        const EN_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/i18n/en.json"));
        const ES_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/i18n/es.json"));

        $crate::validate_key!(EN_JSON, ES_JSON, $key);

        $crate::i18n::display::t_params_lang($key, $params, $lang)
    }};
}

/// Macro for call-site validation - this will show errors at the call site
#[macro_export]
macro_rules! validate_key {
    ($en_json:expr, $es_json:expr, $key:literal) => {
        const _: () = {
            const EN_PATTERN: [u8; 128] = $crate::i18n::macros::create_json_key_pattern($key);
            const ES_HAS_KEY: bool = $crate::i18n::macros::contains_pattern($es_json, &EN_PATTERN);
            const EN_HAS_KEY: bool = $crate::i18n::macros::contains_pattern($en_json, &EN_PATTERN);

            if !EN_HAS_KEY {
                panic!(concat!("Translation key '", $key, "' not found in en.json"));
            }
            if !ES_HAS_KEY {
                panic!(concat!("Translation key '", $key, "' not found in es.json"));
            }
        };
    };
}

/// Create a JSON key pattern like "key": for searching
///
/// # Notes
///
/// This is a rudimentary implementation and may not be perfect. It only supports one level of
/// nesting but that should be enough for our purposes.
pub const fn create_json_key_pattern(key: &str) -> [u8; 128] {
    let mut pattern = [0u8; 128];
    let mut pos = 0;

    pattern[pos] = b'"';
    pos += 1;

    let key_bytes = key.as_bytes();
    let mut i = 0;
    while i < key_bytes.len() && pos < 126 {
        pattern[pos] = key_bytes[i];
        pos += 1;
        i += 1;
    }

    if pos < 127 {
        pattern[pos] = b'"';
        pos += 1;
    }
    if pos < 128 {
        pattern[pos] = b':';
    }

    pattern
}

/// Check if a pattern exists in the text
pub const fn contains_pattern(text: &str, pattern: &[u8; 128]) -> bool {
    let text_bytes = text.as_bytes();

    let mut pattern_len = 0;
    while pattern_len < 128 && pattern[pattern_len] != 0 {
        pattern_len += 1;
    }

    if pattern_len == 0 || pattern_len > text_bytes.len() {
        return false;
    }

    let mut i = 0;
    while i <= text_bytes.len() - pattern_len {
        let mut j = 0;
        let mut matches = true;

        while j < pattern_len {
            if text_bytes[i + j] != pattern[j] {
                matches = false;
                break;
            }
            j += 1;
        }

        if matches {
            return true;
        }
        i += 1;
    }

    false
}
