//! Backup archive creation, listing, download, and metadata helpers.
//!
//! Go's backup service zips the whole cluster directory with the cluster
//! directory name as the first archive component. This module preserves that
//! archive shape while keeping path validation behind safe filesystem helpers.

use std::{
    fs,
    io::{self, Read},
    path::{Component, Path, PathBuf},
};

use chrono::{DateTime, Local, Utc};
use serde::Serialize;
use serde_json::{Map, Number, Value, json};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{
    dst::{self, DstConfig, cluster_ini::ClusterIni},
    infra::fs_paths::{
        self, safe_create_new_file_under_base, safe_directory_exists_under_base,
        safe_open_existing_file_under_base, safe_open_optional_existing_file_under_base,
        safe_remove_file_under_base, safe_rename_file_under_base,
    },
    validation::{
        ValidationError, validate_backup_archive_name, validate_cluster_name, validate_filename,
    },
    web::error::{AppError, AppResult},
};

const MAX_ARCHIVE_META_BYTES: u64 = 512 * 1024;

/// Go `BackupVo` response shape for backup list endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct BackupEntry {
    #[serde(rename = "createTime")]
    pub create_time: DateTime<Utc>,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "fileSize")]
    pub file_size: i64,
    pub time: i64,
}

/// Creates a Go-shaped zip backup for `cluster_name` and returns the file name.
pub(crate) fn create_cluster_backup(
    root: &Path,
    cluster_name: &str,
    backup_name: Option<&str>,
) -> AppResult<String> {
    let config = DstConfig::load(root).map_err(file_error("load dst_config"))?;
    let cluster_name = validate_cluster_name(cluster_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?
        .into_string();
    let backup_name = match backup_name {
        Some(name) => validate_backup_archive_name(name)
            .map_err(|error| AppError::bad_request(error.to_string()))?
            .into_string(),
        None => generate_game_backup_name(root, &config, &cluster_name)?,
    };
    let backup_dir = Path::new(&config.backup);
    let temp_name = temporary_backup_name();
    let write_result = write_cluster_backup_zip(
        backup_dir,
        &temp_name,
        &config.klei_root(root),
        &cluster_name,
    );
    if let Err(error) = write_result {
        let _ = safe_remove_file_under_base(backup_dir, &temp_name);
        return Err(file_error("create backup archive")(error));
    }
    if let Err(error) = safe_rename_file_under_base(backup_dir, &temp_name, &backup_name) {
        let _ = safe_remove_file_under_base(backup_dir, &temp_name);
        return Err(fs_bad_request(error));
    }
    tracing::info!(
        cluster_name,
        backup_name,
        "created DST cluster backup archive"
    );
    Ok(backup_name)
}

/// Lists Go-visible backup archives for the current cluster.
pub(crate) fn list_cluster_backups(root: &Path, cluster_name: &str) -> AppResult<Vec<BackupEntry>> {
    let config = DstConfig::load(root).map_err(file_error("load dst_config"))?;
    let _ = validate_cluster_name(cluster_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?;
    let mut backups = Vec::new();
    let backup_dir = Path::new(&config.backup);
    let entries = match fs::read_dir(backup_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(file_error("read backup directory")(error)),
    };
    for entry in entries {
        let entry = entry.map_err(file_error("read backup entry"))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_listed_backup_archive(&name) {
            continue;
        }
        let Some(mut file) = safe_open_optional_existing_file_under_base(backup_dir, &name)
            .map_err(fs_bad_request)?
        else {
            continue;
        };
        let metadata = file
            .metadata()
            .map_err(file_error("read backup metadata"))?;
        // Touch the descriptor so symlink/non-file leaves cannot be raced after
        // the metadata check. The file remains opened only inside this loop.
        let mut one_byte = [0_u8; 1];
        let _ = file.read(&mut one_byte);
        let modified = metadata
            .modified()
            .map_err(file_error("read backup modified time"))?;
        let create_time: DateTime<Utc> = modified.into();
        backups.push(BackupEntry {
            create_time,
            file_name: name,
            file_size: i64::try_from(metadata.len()).unwrap_or(i64::MAX),
            time: create_time.timestamp(),
        });
    }
    backups.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    Ok(backups)
}

/// Renames one backup archive without replacing an existing destination.
pub(crate) fn rename_cluster_backup(root: &Path, file_name: &str, new_name: &str) -> AppResult<()> {
    let config = DstConfig::load(root).map_err(file_error("load dst_config"))?;
    let file_name = validate_backup_file_name(file_name)?;
    let new_name = validate_backup_file_name(new_name)?;
    safe_rename_file_under_base(Path::new(&config.backup), &file_name, &new_name)
        .map_err(fs_bad_request)?;
    tracing::info!(file_name, new_name, "renamed backup archive");
    Ok(())
}

/// Deletes selected backup archives, ignoring missing files like Go.
pub(crate) fn delete_cluster_backups(root: &Path, file_names: &[String]) -> AppResult<()> {
    let config = DstConfig::load(root).map_err(file_error("load dst_config"))?;
    for raw_name in file_names {
        let file_name = validate_backup_file_name(raw_name)?;
        let removed = safe_remove_file_under_base(Path::new(&config.backup), &file_name)
            .map_err(fs_bad_request)?;
        tracing::info!(file_name, removed, "processed backup delete request");
    }
    Ok(())
}

/// Opens a backup archive for raw download streaming.
pub(crate) fn open_cluster_backup(root: &Path, file_name: &str) -> AppResult<(String, fs::File)> {
    let config = DstConfig::load(root).map_err(file_error("load dst_config"))?;
    let file_name = validate_backup_file_name(file_name)?;
    let file = safe_open_existing_file_under_base(Path::new(&config.backup), &file_name)
        .map_err(fs_bad_request)?;
    tracing::info!(file_name, "opened backup archive for download");
    Ok((file_name, file))
}

fn temporary_backup_name() -> String {
    format!(
        ".dst-admin-rust-backup-{}-{}.zip.tmp",
        std::process::id(),
        Local::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| Local::now().timestamp_micros())
    )
}

fn validate_backup_file_name(value: &str) -> AppResult<String> {
    let name = validate_backup_archive_name(value)
        .map_err(|error| AppError::bad_request(error.to_string()))?
        .into_string();
    if !is_listed_backup_archive(&name) {
        return Err(AppError::bad_request("backup archive must be .zip or .tar"));
    }
    Ok(name)
}

fn is_listed_backup_archive(name: &str) -> bool {
    name.ends_with(".zip") || name.ends_with(".tar")
}

fn write_cluster_backup_zip(
    backup_dir: &Path,
    temp_name: &str,
    klei_root: &Path,
    cluster_name: &str,
) -> io::Result<()> {
    let backup_file =
        safe_create_new_file_under_base(backup_dir, temp_name).map_err(fs_path_error)?;
    let mut zip = ZipWriter::new(backup_file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip_cluster_entries(&mut zip, options, klei_root, cluster_name, Path::new(""))?;
    zip.finish().map_err(zip_error)?;
    Ok(())
}

fn generate_game_backup_name(
    root: &Path,
    config: &DstConfig,
    cluster_name: &str,
) -> AppResult<String> {
    let display_name = read_cluster_display_name(root, config, cluster_name)?;
    let archive_desc = read_archive_meta(root, config, cluster_name).archive_description();
    // Go uses Chinese date separators in `2006年01月02日15点04分05秒`.
    let timestamp =
        Local::now().format("%Y\u{5e74}%m\u{6708}%d\u{65e5}%H\u{70b9}%M\u{5206}%S\u{79d2}");
    let archive_name = format!("{timestamp}_{display_name}_{archive_desc}.zip");
    validate_backup_archive_name(&archive_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?;
    Ok(archive_name)
}

fn read_cluster_display_name(
    root: &Path,
    config: &DstConfig,
    cluster_name: &str,
) -> AppResult<String> {
    let cluster_dir = config.klei_root(root).join(cluster_name);
    let contents = dst::safe_read_cluster_file_to_string(&cluster_dir, "cluster.ini")
        .map_err(file_error("read cluster.ini"))?;
    let display_name = contents
        .as_deref()
        .map(ClusterIni::from_contents)
        .map(|cluster_ini| cluster_ini.cluster_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| cluster_name.to_owned());
    Ok(display_name)
}

pub(crate) fn archive_meta_value(root: &Path, config: &DstConfig, cluster_name: &str) -> Value {
    read_archive_meta(root, config, cluster_name).to_go_value()
}

#[derive(Debug, Clone, Default)]
struct ArchiveMeta {
    clock: ArchiveClock,
    seasons: ArchiveSeasons,
}

#[derive(Debug, Clone, Default)]
struct ArchiveClock {
    total_time_in_phase: i64,
    cycles: i64,
    phase: String,
    remaining_time_in_phase: f64,
    mooom_phase_cycle: i64,
    segs: ArchiveSegs,
}

#[derive(Debug, Clone, Default)]
struct ArchiveSegs {
    night: i64,
    day: i64,
    dusk: i64,
}

#[derive(Debug, Clone)]
struct ArchiveSeasons {
    premode: bool,
    season: String,
    elapsed_days_in_season: i64,
    is_random: ArchiveSeasonFlags,
    lengths: ArchiveSeasonLengths,
    remaining_days_in_season: i64,
    mode: String,
    total_days_in_season: i64,
    segs: Value,
}

impl Default for ArchiveSeasons {
    fn default() -> Self {
        Self {
            premode: false,
            season: String::new(),
            elapsed_days_in_season: 0,
            is_random: ArchiveSeasonFlags::default(),
            lengths: ArchiveSeasonLengths::default(),
            remaining_days_in_season: 0,
            mode: String::new(),
            total_days_in_season: 0,
            segs: Value::Null,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ArchiveSeasonFlags {
    summer: bool,
    autumn: bool,
    spring: bool,
    winter: bool,
}

#[derive(Debug, Clone, Default)]
struct ArchiveSeasonLengths {
    summer: i64,
    autumn: i64,
    spring: i64,
    winter: i64,
}

impl ArchiveMeta {
    fn archive_description(&self) -> String {
        let elapsed = self.seasons.elapsed_days_in_season;
        let total = elapsed + self.seasons.remaining_days_in_season;
        format!(
            "{}day_{}_{}({}_{})",
            self.clock.cycles,
            self.clock.phase,
            localized_season(&self.seasons.season),
            elapsed,
            total
        )
    }

    fn to_go_value(&self) -> Value {
        json!({
            "Clock": {
                "TotalTimeInPhase": self.clock.total_time_in_phase,
                "Cycles": self.clock.cycles,
                "Phase": self.clock.phase,
                "RemainingTimeInPhase": self.clock.remaining_time_in_phase,
                "MooomPhaseCycle": self.clock.mooom_phase_cycle,
                "Segs": {
                    "Night": self.clock.segs.night,
                    "Day": self.clock.segs.day,
                    "Dusk": self.clock.segs.dusk,
                }
            },
            "Seasons": {
                "Premode": self.seasons.premode,
                "Season": self.seasons.season,
                "ElapsedDaysInSeason": self.seasons.elapsed_days_in_season,
                "IsRandom": {
                    "Summer": self.seasons.is_random.summer,
                    "Autumn": self.seasons.is_random.autumn,
                    "Spring": self.seasons.is_random.spring,
                    "Winter": self.seasons.is_random.winter,
                },
                "Lengths": {
                    "Summer": self.seasons.lengths.summer,
                    "Autumn": self.seasons.lengths.autumn,
                    "Spring": self.seasons.lengths.spring,
                    "Winter": self.seasons.lengths.winter,
                },
                "RemainingDaysInSeason": self.seasons.remaining_days_in_season,
                "Mode": self.seasons.mode,
                "TotalDaysInSeason": self.seasons.total_days_in_season,
                "Segs": self.seasons.segs,
            }
        })
    }
}

fn localized_season(season: &str) -> &'static str {
    match season {
        "spring" => "春天",
        "summer" => "夏天",
        "autumn" => "秋天",
        "winter" => "冬天",
        _ => "",
    }
}

fn read_archive_meta(root: &Path, config: &DstConfig, cluster_name: &str) -> ArchiveMeta {
    match read_latest_archive_meta(root, config, cluster_name) {
        Ok(Some(meta)) => meta,
        Ok(None) => ArchiveMeta::default(),
        Err(error) => {
            tracing::warn!(
                cluster_name,
                error = %error,
                "failed to read DST archive meta; using Go zero-value meta"
            );
            ArchiveMeta::default()
        }
    }
}

fn read_latest_archive_meta(
    root: &Path,
    config: &DstConfig,
    cluster_name: &str,
) -> io::Result<Option<ArchiveMeta>> {
    let cluster_dir = config.klei_root(root).join(cluster_name);
    let Some(relative_path) = latest_meta_relative_path(&cluster_dir)? else {
        return Ok(None);
    };
    let mut file =
        safe_open_existing_file_under_base(&cluster_dir, &relative_path).map_err(fs_path_error)?;
    let size = file
        .metadata()
        .map_err(|_| io::Error::other("archive meta is unavailable"))?
        .len();
    if size > MAX_ARCHIVE_META_BYTES {
        tracing::warn!(
            cluster_name,
            size,
            max_size = MAX_ARCHIVE_META_BYTES,
            "DST archive meta is too large; using Go zero-value meta"
        );
        return Ok(None);
    }
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(parse_archive_meta(&contents))
}

fn latest_meta_relative_path(cluster_dir: &Path) -> io::Result<Option<PathBuf>> {
    let session_relative = Path::new("Master").join("save").join("session");
    if !safe_directory_exists_under_base(cluster_dir, &session_relative).map_err(fs_path_error)? {
        return Ok(None);
    }
    let session_dir = cluster_dir.join(&session_relative);
    let mut latest: Option<(std::time::SystemTime, PathBuf)> = None;
    for subdir in fs::read_dir(&session_dir)? {
        let subdir = subdir?;
        let file_type = subdir.file_type()?;
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        for file in fs::read_dir(subdir.path())? {
            let file = file?;
            let file_type = file.file_type()?;
            if !file_type.is_file() || file_type.is_symlink() {
                continue;
            }
            if file.path().extension().and_then(|value| value.to_str()) != Some("meta") {
                continue;
            }
            let modified = file.metadata()?.modified().unwrap_or(std::time::UNIX_EPOCH);
            let relative = session_relative
                .join(subdir.file_name())
                .join(file.file_name());
            if latest
                .as_ref()
                .is_none_or(|(latest_modified, _)| modified > *latest_modified)
            {
                latest = Some((modified, relative));
            }
        }
    }
    Ok(latest.map(|(_, path)| path))
}

fn parse_archive_meta(contents: &str) -> Option<ArchiveMeta> {
    // DST `.meta` files are Lua table literals. The Go code executes them through
    // a Lua VM; Rust deliberately parses only the small field subset needed for
    // `vo.Meta` so an uploaded backup cannot execute code during archive reads.
    let root = LuaMetaParser::new(contents).parse_root().ok()?;
    let clock = root.field("clock");
    let clock_segs = clock.and_then(|value| value.field("segs"));
    let seasons = root.field("seasons");
    let is_random = seasons.and_then(|value| value.field("israndom"));
    let lengths = seasons.and_then(|value| value.field("lengths"));
    let season_segs = seasons
        .and_then(|value| value.field("segs"))
        .and_then(lua_value_to_json)
        .unwrap_or(Value::Null);

    Some(ArchiveMeta {
        clock: ArchiveClock {
            total_time_in_phase: lua_i64(clock, "totaltimeinphase"),
            cycles: lua_i64(clock, "cycles"),
            phase: lua_string(clock, "phase"),
            remaining_time_in_phase: lua_f64(clock, "remainingtimeinphase"),
            mooom_phase_cycle: lua_i64(clock, "mooomphasecycle"),
            segs: ArchiveSegs {
                night: lua_i64(clock_segs, "night"),
                day: lua_i64(clock_segs, "day"),
                dusk: lua_i64(clock_segs, "dusk"),
            },
        },
        seasons: ArchiveSeasons {
            premode: lua_bool(seasons, "premode"),
            season: lua_string(seasons, "season"),
            elapsed_days_in_season: lua_i64(seasons, "elapseddaysinseason"),
            is_random: ArchiveSeasonFlags {
                summer: lua_bool(is_random, "summer"),
                autumn: lua_bool(is_random, "autumn"),
                spring: lua_bool(is_random, "spring"),
                winter: lua_bool(is_random, "winter"),
            },
            lengths: ArchiveSeasonLengths {
                summer: lua_i64(lengths, "summer"),
                autumn: lua_i64(lengths, "autumn"),
                spring: lua_i64(lengths, "spring"),
                winter: lua_i64(lengths, "winter"),
            },
            remaining_days_in_season: lua_i64(seasons, "remainingdaysinseason"),
            mode: lua_string(seasons, "mode"),
            total_days_in_season: lua_i64(seasons, "totaldaysinseason"),
            segs: season_segs,
        },
    })
}

fn lua_i64(table: Option<&LuaValue>, key: &str) -> i64 {
    table
        .and_then(|value| value.field(key))
        .and_then(LuaValue::as_i64)
        .unwrap_or_default()
}

fn lua_f64(table: Option<&LuaValue>, key: &str) -> f64 {
    table
        .and_then(|value| value.field(key))
        .and_then(LuaValue::as_f64)
        .unwrap_or_default()
}

fn lua_bool(table: Option<&LuaValue>, key: &str) -> bool {
    table
        .and_then(|value| value.field(key))
        .and_then(LuaValue::as_bool)
        .unwrap_or_default()
}

fn lua_string(table: Option<&LuaValue>, key: &str) -> String {
    table
        .and_then(|value| value.field(key))
        .and_then(LuaValue::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn lua_value_to_json(value: &LuaValue) -> Option<Value> {
    match value {
        LuaValue::Table(fields) => {
            let mut object = Map::new();
            for (key, value) in fields {
                object.insert(key.clone(), lua_value_to_json(value).unwrap_or(Value::Null));
            }
            Some(Value::Object(object))
        }
        LuaValue::String(value) => Some(Value::String(value.clone())),
        LuaValue::Number(value) => Number::from_f64(*value).map(Value::Number),
        LuaValue::Bool(value) => Some(Value::Bool(*value)),
    }
}

#[derive(Debug, Clone)]
enum LuaValue {
    Table(Vec<(String, LuaValue)>),
    String(String),
    Number(f64),
    Bool(bool),
}

impl LuaValue {
    fn field(&self, key: &str) -> Option<&LuaValue> {
        let Self::Table(fields) = self else {
            return None;
        };
        fields
            .iter()
            .rev()
            .find_map(|(field_key, value)| (field_key == key).then_some(value))
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(value) => Some(*value as i64),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }
}

struct LuaMetaParser<'a> {
    input: &'a str,
    offset: usize,
    depth: usize,
}

impl<'a> LuaMetaParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            offset: 0,
            depth: 0,
        }
    }

    fn parse_root(mut self) -> Result<LuaValue, ()> {
        self.skip_ws_and_comments();
        self.consume_word("return");
        self.skip_ws_and_comments();
        let value = self.parse_value()?;
        Ok(value)
    }

    fn parse_value(&mut self) -> Result<LuaValue, ()> {
        self.skip_ws_and_comments();
        if self.consume_char('{') {
            return self.parse_table();
        }
        if self.peek_char().is_some_and(|ch| ch == '"' || ch == '\'') {
            return self.parse_string().map(LuaValue::String);
        }
        if self.consume_word("true") {
            return Ok(LuaValue::Bool(true));
        }
        if self.consume_word("false") {
            return Ok(LuaValue::Bool(false));
        }
        self.parse_number().map(LuaValue::Number)
    }

    fn parse_table(&mut self) -> Result<LuaValue, ()> {
        self.depth += 1;
        if self.depth > 16 {
            return Err(());
        }
        let mut fields = Vec::new();
        loop {
            self.skip_ws_and_comments();
            if self.consume_char('}') {
                self.depth -= 1;
                return Ok(LuaValue::Table(fields));
            }
            let start = self.offset;
            if let Some(key) = self.parse_table_key() {
                self.skip_ws_and_comments();
                if self.consume_char('=') {
                    let value = self.parse_value()?;
                    fields.push((key, value));
                    self.consume_separator();
                    continue;
                }
            }
            self.offset = start;
            let _ = self.parse_value()?;
            self.consume_separator();
        }
    }

    fn parse_table_key(&mut self) -> Option<String> {
        self.skip_ws_and_comments();
        if self.consume_char('[') {
            self.skip_ws_and_comments();
            let key = if self.peek_char().is_some_and(|ch| ch == '"' || ch == '\'') {
                self.parse_string().ok()?
            } else {
                self.parse_identifier()?
            };
            self.skip_ws_and_comments();
            if !self.consume_char(']') {
                return None;
            }
            return Some(key);
        }
        self.parse_identifier()
    }

    fn parse_identifier(&mut self) -> Option<String> {
        self.skip_ws_and_comments();
        let mut end = self.offset;
        for (index, ch) in self.input[self.offset..].char_indices() {
            if index == 0 {
                if !(ch == '_' || ch.is_ascii_alphabetic()) {
                    return None;
                }
            } else if !(ch == '_' || ch.is_ascii_alphanumeric()) {
                break;
            }
            end = self.offset + index + ch.len_utf8();
        }
        if end == self.offset {
            return None;
        }
        let value = self.input[self.offset..end].to_owned();
        self.offset = end;
        Some(value)
    }

    fn parse_string(&mut self) -> Result<String, ()> {
        let quote = self.next_char().ok_or(())?;
        if quote != '"' && quote != '\'' {
            return Err(());
        }
        let mut value = String::new();
        while let Some(ch) = self.next_char() {
            if ch == quote {
                return Ok(value);
            }
            if ch == '\\' {
                let escaped = self.next_char().ok_or(())?;
                value.push(match escaped {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '"' => '"',
                    '\'' => '\'',
                    '\\' => '\\',
                    other => other,
                });
            } else {
                value.push(ch);
            }
        }
        Err(())
    }

    fn parse_number(&mut self) -> Result<f64, ()> {
        self.skip_ws_and_comments();
        let start = self.offset;
        if self.peek_char() == Some('-') {
            self.next_char();
        }
        let mut seen_digit = false;
        while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            seen_digit = true;
            self.next_char();
        }
        if self.peek_char() == Some('.') {
            self.next_char();
            while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
                seen_digit = true;
                self.next_char();
            }
        }
        if !seen_digit {
            self.offset = start;
            return Err(());
        }
        self.input[start..self.offset]
            .parse::<f64>()
            .map_err(|_| ())
    }

    fn consume_separator(&mut self) {
        self.skip_ws_and_comments();
        if self.peek_char().is_some_and(|ch| ch == ',' || ch == ';') {
            self.next_char();
        }
    }

    fn consume_word(&mut self, word: &str) -> bool {
        self.skip_ws_and_comments();
        let rest = &self.input[self.offset..];
        if !rest.starts_with(word) {
            return false;
        }
        let after = self.offset + word.len();
        if self.input[after..]
            .chars()
            .next()
            .is_some_and(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        {
            return false;
        }
        self.offset = after;
        true
    }

    fn consume_char(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.next_char();
            true
        } else {
            false
        }
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while self.peek_char().is_some_and(char::is_whitespace) {
                self.next_char();
            }
            if self.input[self.offset..].starts_with("--") {
                while self.peek_char().is_some_and(|ch| ch != '\n') {
                    self.next_char();
                }
                continue;
            }
            break;
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.offset..].chars().next()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }
}

fn zip_cluster_entries(
    zip: &mut ZipWriter<fs::File>,
    options: SimpleFileOptions,
    klei_root: &Path,
    cluster_name: &str,
    relative_dir: &Path,
) -> io::Result<()> {
    let absolute_dir = klei_root.join(cluster_name).join(relative_dir);
    for entry in fs::read_dir(&absolute_dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cluster backup must not follow symlinks",
            ));
        }
        let archive_path = Path::new(cluster_name)
            .join(relative_dir)
            .join(entry.file_name());
        let relative_path = relative_dir.join(entry.file_name());
        if metadata.is_dir() {
            zip_cluster_entries(zip, options, klei_root, cluster_name, &relative_path)?;
        } else if metadata.is_file() {
            let archive_name = safe_archive_name(&archive_path)?;
            let mut file = safe_open_existing_file_under_base(
                klei_root,
                Path::new(cluster_name).join(&relative_path),
            )
            .map_err(fs_path_error)?;
            zip.start_file(archive_name, options).map_err(zip_error)?;
            io::copy(&mut file, zip)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cluster backup contains unsupported file type",
            ));
        }
    }
    Ok(())
}

fn safe_archive_name(relative_path: &Path) -> io::Result<String> {
    let mut parts = Vec::new();
    for component in relative_path.components() {
        let Component::Normal(value) = component else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cluster backup path contains unsafe components",
            ));
        };
        let value = value.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "cluster backup path contains non-utf8 component",
            )
        })?;
        validate_filename(value).map_err(validation_error)?;
        parts.push(value.to_owned());
    }
    if parts.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "cluster backup path cannot be empty",
        ));
    }
    Ok(parts.join("/"))
}

fn validation_error(error: ValidationError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

fn fs_path_error(error: fs_paths::FsPathError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

fn fs_bad_request(error: fs_paths::FsPathError) -> AppError {
    tracing::warn!(error = %error, "rejected unsafe backup path");
    AppError::bad_request(error.to_string())
}

fn zip_error(error: zip::result::ZipError) -> io::Error {
    io::Error::other(error)
}

fn file_error(operation: &'static str) -> impl FnOnce(io::Error) -> AppError {
    move |error| {
        tracing::error!(operation, error = %error, "backup filesystem operation failed");
        if error.kind() == io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            AppError::internal(operation)
        }
    }
}
