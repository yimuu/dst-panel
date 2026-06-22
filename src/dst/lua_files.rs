//! Helpers for DST Lua config files that Go reads and writes as raw text.

/// Converts optional Lua file contents into Go's `return {}` default.
pub fn contents_or_default(contents: Option<String>) -> String {
    let Some(contents) = contents else {
        return "return {}".to_owned();
    };
    if contents.is_empty() {
        "return {}".to_owned()
    } else {
        contents
    }
}
