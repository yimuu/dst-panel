//! World session-file discovery and reading helpers.

use std::{
    fs, io,
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    dst,
    infra::fs_paths::safe_directory_exists_under_base,
    validation::{LevelName, validate_level_name},
};

const MAX_SESSION_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SESSION_DIRECTORY_ENTRIES: usize = 4096;

/// Validated request data for routes that operate on a single DST level.
#[derive(Debug, Clone)]
pub struct MapLevel {
    level_name: LevelName,
}

impl MapLevel {
    /// Validates a level name before it is used as a cluster-relative path.
    pub fn parse(level_name: &str) -> io::Result<Self> {
        let level_name = validate_level_name(level_name)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        Ok(Self { level_name })
    }

    /// Returns the validated level name as a path-safe string.
    pub fn as_str(&self) -> &str {
        self.level_name.as_str()
    }
}

/// Returns the raw text from the newest world session file for `level`.
pub fn read_latest_session_file(cluster_dir: &Path, level: &MapLevel) -> io::Result<String> {
    let relative_path = latest_world_session_file(cluster_dir, level)?;
    read_cluster_text(cluster_dir, &relative_path)
}

/// Returns whether the latest world session file contains Go's hard-coded
/// `WalrusHut_Plains` marker.
pub fn latest_session_has_walrus_hut_plains(
    cluster_dir: &Path,
    level: &MapLevel,
) -> io::Result<bool> {
    read_latest_session_file(cluster_dir, level)
        .map(|contents| contents.contains("WalrusHut_Plains"))
}

pub(super) fn latest_world_session_file(
    cluster_dir: &Path,
    level: &MapLevel,
) -> io::Result<PathBuf> {
    let session_relative = session_base_relative(level);
    latest_file_in_child_dirs(
        cluster_dir,
        &session_relative,
        LatestFileFilter::ExcludeMetaExtension,
    )
}

pub(super) fn session_base_relative(level: &MapLevel) -> PathBuf {
    Path::new(level.as_str()).join("save").join("session")
}

#[derive(Debug, Clone, Copy)]
pub(super) enum LatestFileFilter {
    Any,
    ExcludeMetaExtension,
}

fn latest_file_in_child_dirs(
    cluster_dir: &Path,
    directory_relative: &Path,
    filter: LatestFileFilter,
) -> io::Result<PathBuf> {
    let directory = checked_session_directory(cluster_dir, directory_relative)?;
    let mut latest: Option<(SystemTime, PathBuf)> = None;
    // Go used ioutil.ReadDir, which returns filename-sorted entries. Keep the
    // same deterministic traversal so equal-mtime files pick the first sorted
    // candidate because `consider_latest_file` updates only on strictly newer
    // modification times. Go's world-session helper only descends into direct
    // child directories, so direct files in `save/session` are ignored here.
    for child in sorted_read_dir(&directory)? {
        let child_type = child.file_type()?;
        if child_type.is_symlink() {
            tracing::warn!(
                path = %child.path().display(),
                "skipping symlink while scanning DST session directory"
            );
            continue;
        }

        if child_type.is_dir() {
            scan_session_child_dir(cluster_dir, directory_relative, child, filter, &mut latest)?;
        }
    }

    latest
        .map(|(_, path)| path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到文件"))
}

pub(super) fn latest_direct_file_in_dir(
    cluster_dir: &Path,
    directory_relative: &Path,
    filter: LatestFileFilter,
) -> io::Result<PathBuf> {
    let directory = checked_session_directory(cluster_dir, directory_relative)?;
    let mut latest: Option<(SystemTime, PathBuf)> = None;

    // Go's player-session helper only looks at direct files in the `${kuId}_`
    // directory. Nested directories are ignored even if they contain newer save
    // files, which matters for compatibility on copied or partially restored
    // save trees.
    for file in sorted_read_dir(&directory)? {
        let file_type = file.file_type()?;
        if file_type.is_symlink() {
            tracing::warn!(
                path = %file.path().display(),
                "skipping symlink while scanning direct DST session files"
            );
            continue;
        }
        if file_type.is_file() {
            consider_latest_file(directory_relative, file, filter, &mut latest)?;
        }
    }

    latest
        .map(|(_, path)| path)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到文件"))
}

fn checked_session_directory(cluster_dir: &Path, directory_relative: &Path) -> io::Result<PathBuf> {
    if !safe_directory_exists_under_base(cluster_dir, directory_relative).map_err(fs_path_error)? {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "目录不存在：{}",
                cluster_dir.join(directory_relative).display()
            ),
        ));
    }

    Ok(cluster_dir.join(directory_relative))
}

fn scan_session_child_dir(
    cluster_dir: &Path,
    directory_relative: &Path,
    child: fs::DirEntry,
    filter: LatestFileFilter,
    latest: &mut Option<(SystemTime, PathBuf)>,
) -> io::Result<()> {
    let child_name = child.file_name();
    let child_relative = directory_relative.join(&child_name);
    if !safe_directory_exists_under_base(cluster_dir, &child_relative).map_err(fs_path_error)? {
        tracing::warn!(
            path = %child.path().display(),
            "skipping unsafe DST session subdirectory"
        );
        return Ok(());
    }

    for file in sorted_read_dir(child.path())? {
        let file_type = file.file_type()?;
        if file_type.is_symlink() {
            tracing::warn!(
                path = %file.path().display(),
                "skipping symlink while scanning DST session files"
            );
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        consider_latest_file(&child_relative, file, filter, latest)?;
    }
    Ok(())
}

fn sorted_read_dir(path: impl AsRef<Path>) -> io::Result<Vec<fs::DirEntry>> {
    let path = path.as_ref();
    let mut entries = Vec::new();
    for (index, entry) in fs::read_dir(path)?.enumerate() {
        if index >= MAX_SESSION_DIRECTORY_ENTRIES {
            tracing::warn!(
                path = %path.display(),
                max_entries = MAX_SESSION_DIRECTORY_ENTRIES,
                "refusing oversized DST session directory"
            );
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "session directory exceeds safety limit",
            ));
        }
        entries.push(entry?);
    }
    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries)
}

fn consider_latest_file(
    parent_relative: &Path,
    file: fs::DirEntry,
    filter: LatestFileFilter,
    latest: &mut Option<(SystemTime, PathBuf)>,
) -> io::Result<()> {
    if matches!(filter, LatestFileFilter::ExcludeMetaExtension)
        && file.path().extension().and_then(|value| value.to_str()) == Some("meta")
    {
        return Ok(());
    }

    let modified = file.metadata()?.modified().unwrap_or(UNIX_EPOCH);
    let relative = parent_relative.join(file.file_name());
    if latest
        .as_ref()
        .is_none_or(|(latest_modified, _)| modified > *latest_modified)
    {
        *latest = Some((modified, relative));
    }
    Ok(())
}

pub(super) fn session_id_from_world_relative_path(
    level: &MapLevel,
    relative_path: &Path,
) -> io::Result<String> {
    let prefix = session_base_relative(level);
    let suffix = relative_path.strip_prefix(&prefix).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "session file path is outside the level session directory",
        )
    })?;
    suffix
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing session id"))
}

pub(super) fn read_cluster_text(cluster_dir: &Path, relative_path: &Path) -> io::Result<String> {
    let Some(mut file) = dst::safe_open_cluster_file(cluster_dir, relative_path)? else {
        return Err(io::Error::new(io::ErrorKind::NotFound, "未找到文件"));
    };
    let bytes = read_capped_bytes(
        &mut file,
        MAX_SESSION_FILE_BYTES,
        "session file exceeds safety limit",
    )?;
    String::from_utf8(bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}

pub(super) fn read_capped_bytes(
    file: &mut fs::File,
    limit: u64,
    message: &'static str,
) -> io::Result<Vec<u8>> {
    reject_oversized_file(file, limit, message)?;

    // Re-check while reading as well as before reading. This closes the race
    // where a file grows after metadata validation but before `read_to_end`.
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(io::Error::new(io::ErrorKind::InvalidData, message));
    }
    Ok(bytes)
}

fn fs_path_error(error: crate::infra::fs_paths::FsPathError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

fn reject_oversized_file(file: &fs::File, max_bytes: u64, message: &'static str) -> io::Result<()> {
    let size = file.metadata()?.len();
    if size > max_bytes {
        return Err(io::Error::new(io::ErrorKind::InvalidData, message));
    }
    Ok(())
}
