//! Central validation helpers for user-controlled identifiers and file names.
//!
//! Validation errors intentionally do not echo the rejected value, so handlers
//! can return them directly to clients without leaking secrets from malformed
//! requests.

use thiserror::Error;

/// Error raised when user input fails a shared safety validator.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid {field}: {reason}")]
pub struct ValidationError {
    field: &'static str,
    reason: &'static str,
}

impl ValidationError {
    fn new(field: &'static str, reason: &'static str) -> Self {
        Self { field, reason }
    }
}

macro_rules! validated_string_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(String);

        impl $name {
            fn new(value: String) -> Self {
                Self(value)
            }

            /// Returns the validated value as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consumes the wrapper and returns the validated string.
            pub fn into_string(self) -> String {
                self.0
            }
        }
    };
}

validated_string_type!(
    ClusterName,
    "Validated DST cluster name safe to use as a single path component."
);
validated_string_type!(
    LevelName,
    "Validated DST shard or level name safe to use as a single path component."
);
validated_string_type!(ModId, "Validated Steam Workshop decimal mod id.");
validated_string_type!(KuId, "Validated Klei user id.");
validated_string_type!(
    SafeFilename,
    "Validated single filename safe to use as one path component."
);
validated_string_type!(
    BackupArchiveName,
    "Validated backup archive filename safe to use as one path component."
);
validated_string_type!(
    SafeCommandArg,
    "Validated user-controlled value safe to place in argv value positions."
);

/// Validates a DST cluster name.
pub fn validate_cluster_name(value: &str) -> Result<ClusterName, ValidationError> {
    validate_path_atom("cluster name", value).map(ClusterName::new)
}

/// Validates a DST shard/level name.
pub fn validate_level_name(value: &str) -> Result<LevelName, ValidationError> {
    validate_path_atom("level name", value).map(LevelName::new)
}

/// Validates a Steam Workshop mod id. Go-compatible callers pass decimal ids.
pub fn validate_mod_id(value: &str) -> Result<ModId, ValidationError> {
    if value.is_empty() {
        return Err(ValidationError::new("mod id", "cannot be empty"));
    }
    if value.len() > 20 {
        return Err(ValidationError::new("mod id", "is too long"));
    }
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ValidationError::new("mod id", "must contain digits only"));
    }
    Ok(ModId::new(value.to_owned()))
}

/// Validates a Klei user id such as `KU_abc-123`.
pub fn validate_ku_id(value: &str) -> Result<KuId, ValidationError> {
    let suffix = value
        .strip_prefix("KU_")
        .ok_or_else(|| ValidationError::new("KU id", "must start with KU_"))?;
    if suffix.is_empty() {
        return Err(ValidationError::new("KU id", "missing id after prefix"));
    }
    if suffix.len() > 128 {
        return Err(ValidationError::new("KU id", "is too long"));
    }
    if !suffix
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(ValidationError::new("KU id", "contains unsafe characters"));
    }
    Ok(KuId::new(value.to_owned()))
}

/// Validates a single file name, not a nested path.
pub fn validate_filename(value: &str) -> Result<SafeFilename, ValidationError> {
    validate_path_atom("filename", value).map(SafeFilename::new)
}

/// Validates a backup archive file name.
pub fn validate_backup_archive_name(value: &str) -> Result<BackupArchiveName, ValidationError> {
    validate_path_atom("backup archive name", value).map(BackupArchiveName::new)
}

/// Validates a user-controlled value before it is passed as an argv value.
///
/// This validator is intentionally stricter than path atom validation: it
/// rejects leading `-` and `+` so future command builders do not accidentally
/// turn a user value into an option, SteamCMD command, or console directive.
pub fn validate_safe_command_arg(
    field: &'static str,
    value: &str,
) -> Result<SafeCommandArg, ValidationError> {
    let value = validate_path_atom(field, value)?;
    if value.starts_with('-') || value.starts_with('+') {
        return Err(ValidationError::new(
            field,
            "cannot start with an option marker",
        ));
    }
    if value
        .chars()
        .any(|ch| matches!(ch, ';' | '&' | '`' | '$' | '(' | ')' | '\''))
    {
        return Err(ValidationError::new(field, "contains unsafe characters"));
    }
    Ok(SafeCommandArg::new(value))
}

fn validate_path_atom(field: &'static str, value: &str) -> Result<String, ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::new(field, "cannot be empty"));
    }
    if value.trim() != value {
        return Err(ValidationError::new(
            field,
            "cannot have surrounding whitespace",
        ));
    }
    if value == "." || value == ".." {
        return Err(ValidationError::new(field, "reserved path component"));
    }
    if value.len() > 255 {
        return Err(ValidationError::new(field, "is too long"));
    }
    if value.ends_with('.') || value.ends_with(' ') {
        return Err(ValidationError::new(
            field,
            "cannot end with dot or whitespace",
        ));
    }
    if is_windows_reserved_name(value) {
        return Err(ValidationError::new(field, "uses a reserved name"));
    }
    if value
        .chars()
        .any(|ch| ch == '/' || ch == '\\' || ch.is_control() || is_windows_invalid_char(ch))
    {
        return Err(ValidationError::new(field, "contains unsafe characters"));
    }
    Ok(value.to_owned())
}

fn is_windows_invalid_char(ch: char) -> bool {
    matches!(ch, ':' | '<' | '>' | '"' | '|' | '?' | '*')
}

fn is_windows_reserved_name(value: &str) -> bool {
    let stem = value.split('.').next().unwrap_or(value);
    let upper = stem.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}
