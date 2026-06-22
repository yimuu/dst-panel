//! Configuration loading and Go-compatible defaults for `config.yml`.
//!
//! The existing Go backend reads camel-case YAML keys into a struct and then
//! fills a small set of missing or zero values. This module preserves those
//! names and defaults while exposing idiomatic Rust field names.

use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Deserializer, de};
use thiserror::Error;
use yaml_serde::Value;

const DEFAULT_DST_VERSION_URL: &str = "https://api.dstserverlist.top/api/v2/Server/Version";
const DEFAULT_MODINFO_CHECK_INTERVAL: i64 = 5;
const DEFAULT_MODINFO_UPDATE_CHECK_INTERVAL: i64 = 10;

/// Runtime configuration loaded from the Go-compatible `config.yml` file.
#[derive(Clone, PartialEq, Eq)]
pub struct AppConfig {
    /// Address passed to the HTTP server bind call.
    pub bind_address: String,
    /// HTTP port stored as a string to match the Go config struct.
    pub port: String,
    /// Optional filesystem path used by legacy deployments.
    pub path: String,
    /// Data directory prefix used by newer Go releases for config and database files.
    pub data_dir: String,
    /// SQLite database path or filename.
    pub database: String,
    /// SteamCMD installation path.
    pub steamcmd: String,
    /// Steam Web API key, if configured.
    pub steam_api_key: Option<String>,
    /// Legacy flag value from `config.yml`.
    pub flag: String,
    /// Public WAN IP override.
    pub wan_ip: String,
    /// Admin IP that can bypass the normal login flow.
    pub white_admin_ip: Option<String>,
    /// Cluster token or other deployment token, if configured.
    pub token: Option<String>,
    /// Endpoint used to check the current DST version.
    pub dst_version_url: String,
    /// Automatic mod-info update settings.
    pub auto_update_modinfo: AutoUpdateModinfoConfig,
    /// Windows helper CLI port stored as a string to match Go.
    pub dst_cli_port: String,
}

impl fmt::Debug for AppConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AppConfig")
            .field("bind_address", &self.bind_address)
            .field("port", &self.port)
            .field("path", &self.path)
            .field("data_dir", &self.data_dir)
            .field("database", &self.database)
            .field("steamcmd", &self.steamcmd)
            .field("steam_api_key", &redacted_option(&self.steam_api_key))
            .field("flag", &self.flag)
            .field("wan_ip", &self.wan_ip)
            .field("white_admin_ip", &self.white_admin_ip)
            .field("token", &redacted_option(&self.token))
            .field("dst_version_url", &self.dst_version_url)
            .field("auto_update_modinfo", &self.auto_update_modinfo)
            .field("dst_cli_port", &self.dst_cli_port)
            .finish()
    }
}

impl AppConfig {
    /// Loads `config.yml` from `path` and applies the defaults used by Go.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let raw = yaml_serde::from_str::<RawAppConfig>(&contents).map_err(|source| {
            ConfigError::Parse {
                path: path.to_path_buf(),
                source,
            }
        })?;
        let config = raw.into_app_config();

        tracing::info!(
            config_path = %path.display(),
            bind_address = %config.bind_address,
            port = %config.port,
            database = %config.database,
            dst_cli_port = %config.dst_cli_port,
            auto_update_modinfo_enabled = config.auto_update_modinfo.enable,
            "loaded application config"
        );

        Ok(config)
    }

    /// Returns the SQLite database path using Go's `dataDir + database` behavior.
    pub fn database_path(&self) -> PathBuf {
        if self.database.starts_with("sqlite:") || Path::new(&self.database).is_absolute() {
            return PathBuf::from(&self.database);
        }

        let data_dir = self.data_dir.trim();
        if data_dir.is_empty() || data_dir == "." || data_dir == "./" {
            PathBuf::from(&self.database)
        } else {
            Path::new(data_dir).join(&self.database)
        }
    }
}

/// Automatic mod-info update settings from `autoUpdateModinfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoUpdateModinfoConfig {
    /// Whether periodic mod-info refresh tasks are enabled.
    pub enable: bool,
    /// Minutes between checks for mod-info changes.
    pub check_interval: i64,
    /// Minutes between mod-info refresh runs.
    pub update_check_interval: i64,
}

/// Errors returned while reading or parsing application configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The configuration file could not be read.
    #[error("failed to read config file `{}`: {source}", path.display())]
    Read {
        /// Path that failed to read.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
    /// The configuration file contained invalid YAML for the expected schema.
    #[error("failed to parse config file `{}`: {source}", path.display())]
    Parse {
        /// Path that failed to parse.
        path: PathBuf,
        /// Underlying YAML parser error.
        #[source]
        source: yaml_serde::Error,
    },
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawAppConfig {
    #[serde(
        rename = "bindAddress",
        alias = "bind_address",
        deserialize_with = "deserialize_string"
    )]
    bind_address: String,
    #[serde(deserialize_with = "deserialize_string")]
    port: String,
    #[serde(deserialize_with = "deserialize_string")]
    path: String,
    #[serde(
        rename = "dataDir",
        alias = "data_dir",
        deserialize_with = "deserialize_string"
    )]
    data_dir: String,
    #[serde(
        rename = "database",
        alias = "db",
        deserialize_with = "deserialize_string"
    )]
    database: String,
    #[serde(deserialize_with = "deserialize_string")]
    steamcmd: String,
    #[serde(
        rename = "steamAPIKey",
        alias = "steam_api_key",
        deserialize_with = "deserialize_optional_string"
    )]
    steam_api_key: Option<String>,
    #[serde(deserialize_with = "deserialize_string")]
    flag: String,
    #[serde(
        rename = "wanip",
        alias = "wanIP",
        alias = "wan_ip",
        deserialize_with = "deserialize_string"
    )]
    wan_ip: String,
    #[serde(
        rename = "whiteadminip",
        alias = "whiteAdminIp",
        alias = "white_admin_ip",
        deserialize_with = "deserialize_optional_string"
    )]
    white_admin_ip: Option<String>,
    #[serde(deserialize_with = "deserialize_optional_string")]
    token: Option<String>,
    #[serde(
        rename = "dstVersionUrl",
        alias = "dst_version_url",
        deserialize_with = "deserialize_string"
    )]
    dst_version_url: String,
    #[serde(rename = "autoUpdateModinfo", alias = "auto_update_modinfo")]
    auto_update_modinfo: RawAutoUpdateModinfoConfig,
    #[serde(
        rename = "dstCliPort",
        alias = "dst_cli_port",
        deserialize_with = "deserialize_string"
    )]
    dst_cli_port: String,
}

impl RawAppConfig {
    fn into_app_config(self) -> AppConfig {
        AppConfig {
            bind_address: self.bind_address,
            port: self.port,
            path: self.path,
            // Go treats an empty dataDir as ./ and then filepath.Join cleans it
            // away for relative database filenames.
            data_dir: default_string(self.data_dir, "./"),
            database: self.database,
            steamcmd: self.steamcmd,
            steam_api_key: self.steam_api_key,
            flag: self.flag,
            wan_ip: self.wan_ip,
            white_admin_ip: self.white_admin_ip,
            token: self.token,
            // Go applies this default only when the configured value is empty.
            dst_version_url: default_string(self.dst_version_url, DEFAULT_DST_VERSION_URL),
            auto_update_modinfo: self.auto_update_modinfo.into_config(),
            dst_cli_port: self.dst_cli_port,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawAutoUpdateModinfoConfig {
    #[serde(deserialize_with = "deserialize_bool")]
    enable: bool,
    #[serde(
        rename = "checkInterval",
        alias = "check_interval",
        deserialize_with = "deserialize_i64"
    )]
    check_interval: i64,
    #[serde(
        rename = "updateCheckInterval",
        alias = "update_check_interval",
        deserialize_with = "deserialize_i64"
    )]
    update_check_interval: i64,
}

impl RawAutoUpdateModinfoConfig {
    fn into_config(self) -> AutoUpdateModinfoConfig {
        AutoUpdateModinfoConfig {
            enable: self.enable,
            // Go treats zero as missing for these scheduler intervals.
            check_interval: default_i64(self.check_interval, DEFAULT_MODINFO_CHECK_INTERVAL),
            update_check_interval: default_i64(
                self.update_check_interval,
                DEFAULT_MODINFO_UPDATE_CHECK_INTERVAL,
            ),
        }
    }
}

fn default_string(value: String, fallback: &str) -> String {
    if value.is_empty() {
        fallback.to_owned()
    } else {
        value
    }
}

fn default_i64(value: i64, fallback: i64) -> i64 {
    if value == 0 { fallback } else { value }
}

fn redacted_option(value: &Option<String>) -> Option<&'static str> {
    value.as_ref().map(|_| "<redacted>")
}

fn deserialize_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<Value>::deserialize(deserializer)? {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(value)) => Ok(value),
        Some(Value::Number(value)) => Ok(value.to_string()),
        Some(Value::Bool(value)) => Ok(value.to_string()),
        // Do not echo the raw YAML value into parse errors. Config parse
        // failures are logged as operational errors, and fields such as
        // `steamAPIKey` and `token` may contain deployment secrets.
        Some(_) => Err(de::Error::custom("expected scalar value")),
    }
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = deserialize_string(deserializer)?;
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn deserialize_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<Value>::deserialize(deserializer)? {
        None | Some(Value::Null) => Ok(0),
        Some(Value::Number(value)) => value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
            .ok_or_else(|| de::Error::custom("expected signed 64-bit integer")),
        Some(Value::String(value)) if value.is_empty() => Ok(0),
        Some(Value::String(value)) => value
            .parse::<i64>()
            .map_err(|source| de::Error::custom(format!("expected integer: {source}"))),
        Some(_) => Err(de::Error::custom("expected integer value")),
    }
}

fn deserialize_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<Value>::deserialize(deserializer)? {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(value)) => Ok(value),
        Some(Value::Number(value)) => Ok(value.as_i64().unwrap_or_default() != 0),
        Some(Value::String(value)) if value.is_empty() => Ok(false),
        Some(Value::String(value)) => value
            .parse::<bool>()
            .map_err(|source| de::Error::custom(format!("expected boolean: {source}"))),
        Some(_) => Err(de::Error::custom("expected boolean value")),
    }
}
