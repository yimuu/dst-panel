//! Backup upload and restore helpers.
//!
//! Uploaded archives are staged through temporary files, and restore operations
//! validate the zip layout before swapping the active cluster directory.

use std::{
    collections::HashSet,
    fs,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
};

use chrono::Local;
use zip::ZipArchive;

use crate::{
    dst::DstConfig,
    infra::fs_paths::{
        safe_create_new_file_under_base, safe_directory_exists_under_base,
        safe_ensure_dir_under_base, safe_open_existing_file_under_base,
        safe_remove_dir_all_under_base, safe_remove_file_under_base, safe_rename_dir_under_base,
        safe_rename_file_under_base,
    },
    validation::{validate_backup_archive_name, validate_cluster_name, validate_filename},
    web::error::{AppError, AppResult},
};

const MAX_RESTORE_UNCOMPRESSED_BYTES: u64 = 128 * 1024 * 1024;
const MAX_RESTORE_ENTRIES: usize = 10_000;
const ZIP_EOCD_SIGNATURE: &[u8; 4] = b"PK\x05\x06";
const ZIP_EOCD_MIN_LEN: usize = 22;
const ZIP_MAX_COMMENT_LEN: usize = u16::MAX as usize;

/// Writes a user-uploaded backup archive without replacing an existing file.
pub(crate) fn begin_cluster_backup_upload(
    root: &Path,
    file_name: &str,
) -> AppResult<BackupUploadSession> {
    let config = DstConfig::load(root).map_err(file_error("load dst_config"))?;
    let file_name = validate_backup_file_name(file_name)?;
    let backup_dir = Path::new(&config.backup);
    let temp_name = temporary_upload_name();
    let file = match safe_create_new_file_under_base(backup_dir, &temp_name) {
        Ok(file) => file,
        Err(error) => return Err(fs_bad_request(error)),
    };
    Ok(BackupUploadSession {
        backup_dir: backup_dir.to_path_buf(),
        temp_name,
        file_name,
        file: Some(file),
        bytes_written: 0,
        committed: false,
    })
}

/// In-progress streamed backup upload.
///
/// The session writes to a hidden temporary file and only renames it to the
/// user-visible archive name on commit. Dropping an uncommitted session removes
/// the temp file, so multipart parse errors and upload size failures cannot
/// leave a partial archive behind.
pub(crate) struct BackupUploadSession {
    backup_dir: PathBuf,
    temp_name: String,
    file_name: String,
    file: Option<fs::File>,
    bytes_written: usize,
    committed: bool,
}

impl BackupUploadSession {
    /// Appends one multipart chunk to the temporary upload file.
    pub(crate) fn write_chunk(&mut self, chunk: &[u8]) -> AppResult<()> {
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| AppError::internal("write uploaded backup"))?;
        file.write_all(chunk)
            .map_err(file_error("write uploaded backup"))?;
        self.bytes_written = self.bytes_written.saturating_add(chunk.len());
        Ok(())
    }

    /// Atomically publishes the uploaded archive if the destination is absent.
    pub(crate) fn commit(mut self) -> AppResult<String> {
        // Close the descriptor before the no-replace rename so Windows-style
        // filesystems and future non-Unix implementations can share semantics.
        let _ = self.file.take();
        if let Err(error) =
            safe_rename_file_under_base(&self.backup_dir, &self.temp_name, &self.file_name)
        {
            let _ = safe_remove_file_under_base(&self.backup_dir, &self.temp_name);
            return Err(fs_bad_request(error));
        }
        self.committed = true;
        tracing::info!(
            file_name = %self.file_name,
            bytes = self.bytes_written,
            "stored uploaded backup archive"
        );
        Ok(self.file_name.clone())
    }
}

impl Drop for BackupUploadSession {
    fn drop(&mut self) {
        if !self.committed {
            let _ = self.file.take();
            let _ = safe_remove_file_under_base(&self.backup_dir, &self.temp_name);
        }
    }
}

/// Restores a backup zip into the selected cluster directory.
pub(crate) fn restore_cluster_backup<F>(
    root: &Path,
    cluster_name: &str,
    backup_name: &str,
    prepare_staged_cluster: F,
) -> AppResult<()>
where
    F: FnOnce(&Path) -> AppResult<()>,
{
    let config = DstConfig::load(root).map_err(file_error("load dst_config"))?;
    let cluster_name = validate_cluster_name(cluster_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?
        .into_string();
    let backup_name = validate_backup_file_name(backup_name)?;
    let backup_dir = Path::new(&config.backup);
    let backup_file =
        safe_open_existing_file_under_base(backup_dir, &backup_name).map_err(fs_bad_request)?;
    let validate_file = backup_file
        .try_clone()
        .map_err(file_error("clone restore archive descriptor"))?;

    let restore_plan = validate_restore_zip(validate_file)?;
    let klei_root = config.klei_root(root);
    let staging_name = temporary_restore_name(&cluster_name);
    if let Err(error) = extract_restore_plan(backup_file, &klei_root, &staging_name, restore_plan) {
        let _ = safe_remove_dir_all_under_base(&klei_root, &staging_name);
        return Err(error);
    }

    let mut staging_guard = RestoreStagingGuard::new(klei_root.clone(), staging_name.clone());
    let tombstone_name = temporary_tombstone_name(&cluster_name);
    let had_existing =
        safe_directory_exists_under_base(&klei_root, &cluster_name).map_err(fs_bad_request)?;
    if had_existing {
        safe_rename_dir_under_base(&klei_root, &cluster_name, &tombstone_name)
            .map_err(fs_bad_request)?;
    }
    if let Err(error) = prepare_staged_cluster(&klei_root.join(&staging_name)) {
        if had_existing
            && let Err(restore_error) =
                safe_rename_dir_under_base(&klei_root, &tombstone_name, &cluster_name)
        {
            tracing::error!(
                cluster_name,
                backup_name,
                error = %restore_error,
                "failed to restore active cluster after staged restore preparation failed"
            );
        }
        return Err(error);
    }
    if let Err(error) = safe_rename_dir_under_base(&klei_root, &staging_name, &cluster_name) {
        if had_existing {
            let _ = safe_rename_dir_under_base(&klei_root, &tombstone_name, &cluster_name);
        }
        return Err(fs_bad_request(error));
    }
    staging_guard.disarm();
    if had_existing && let Err(error) = safe_remove_dir_all_under_base(&klei_root, &tombstone_name)
    {
        // At this point the restored cluster is already active. Matching the
        // HTTP success state to the committed restore is more important than
        // failing the request because cleanup of the old tombstone failed.
        tracing::warn!(
            cluster_name,
            backup_name,
            tombstone_name,
            error = %error,
            "failed to remove restored backup tombstone after commit"
        );
    }
    tracing::info!(cluster_name, backup_name, "restored backup archive");
    Ok(())
}

struct RestoreStagingGuard {
    klei_root: PathBuf,
    staging_name: String,
    active: bool,
}

impl RestoreStagingGuard {
    fn new(klei_root: PathBuf, staging_name: String) -> Self {
        Self {
            klei_root,
            staging_name,
            active: true,
        }
    }

    fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for RestoreStagingGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        if let Err(error) = safe_remove_dir_all_under_base(&self.klei_root, &self.staging_name) {
            tracing::warn!(
                staging_name = %self.staging_name,
                error = %error,
                "failed to clean staged restore directory"
            );
        }
    }
}

fn temporary_upload_name() -> String {
    format!(
        ".dst-admin-rust-upload-{}-{}.tmp",
        std::process::id(),
        Local::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| Local::now().timestamp_micros())
    )
}

fn temporary_restore_name(cluster_name: &str) -> String {
    format!(
        ".dst-admin-rust-restore-{cluster_name}-{}-{}",
        std::process::id(),
        Local::now()
            .timestamp_nanos_opt()
            .unwrap_or_else(|| Local::now().timestamp_micros())
    )
}

fn temporary_tombstone_name(cluster_name: &str) -> String {
    format!(
        ".dst-admin-rust-old-{cluster_name}-{}-{}",
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

struct RestorePlan {
    entries: Vec<RestoreEntry>,
}

struct RestoreEntry {
    index: usize,
    relative_path: PathBuf,
    is_dir: bool,
    size: u64,
}

fn validate_restore_zip<R>(reader: R) -> AppResult<RestorePlan>
where
    R: Read + Seek,
{
    let mut reader = reader;
    preflight_restore_zip_entry_count(&mut reader)?;
    let mut archive =
        ZipArchive::new(reader).map_err(|_| AppError::bad_request("invalid backup zip"))?;
    if archive.len() > MAX_RESTORE_ENTRIES {
        return Err(AppError::payload_too_large(
            "backup zip contains too many entries",
        ));
    }
    let cluster_prefix = find_cluster_prefix(&mut archive)?;
    let mut entries = Vec::new();
    let mut seen_paths = HashSet::new();
    let mut total_uncompressed = 0_u64;
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|_| AppError::bad_request("invalid backup zip entry"))?;
        let Some((relative_path, is_dir, size)) =
            validate_restore_file_entry(&file, &cluster_prefix)?
        else {
            continue;
        };
        if !seen_paths.insert(relative_path.clone()) {
            return Err(AppError::bad_request("backup zip contains duplicate path"));
        }
        if !is_dir {
            total_uncompressed = total_uncompressed.checked_add(size).ok_or_else(|| {
                AppError::payload_too_large("backup zip uncompressed data is too large")
            })?;
            if total_uncompressed > MAX_RESTORE_UNCOMPRESSED_BYTES {
                return Err(AppError::payload_too_large(
                    "backup zip uncompressed data is too large",
                ));
            }
        }
        entries.push(RestoreEntry {
            index,
            relative_path,
            is_dir,
            size,
        });
    }
    Ok(RestorePlan { entries })
}

fn preflight_restore_zip_entry_count<R>(reader: &mut R) -> AppResult<()>
where
    R: Read + Seek,
{
    let file_len = reader
        .seek(SeekFrom::End(0))
        .map_err(file_error("read restore zip end record"))?;
    if file_len < ZIP_EOCD_MIN_LEN as u64 {
        return Err(AppError::bad_request("invalid backup zip"));
    }
    let search_len = file_len.min((ZIP_EOCD_MIN_LEN + ZIP_MAX_COMMENT_LEN) as u64) as usize;
    reader
        .seek(SeekFrom::Start(file_len - search_len as u64))
        .map_err(file_error("seek restore zip end record"))?;
    let mut tail = vec![0_u8; search_len];
    reader
        .read_exact(&mut tail)
        .map_err(file_error("read restore zip end record"))?;

    for offset in (0..=tail.len() - ZIP_EOCD_MIN_LEN).rev() {
        if &tail[offset..offset + ZIP_EOCD_SIGNATURE.len()] != ZIP_EOCD_SIGNATURE {
            continue;
        }
        let comment_len = read_le_u16(&tail[offset + 20..offset + 22]) as usize;
        if offset + ZIP_EOCD_MIN_LEN + comment_len != tail.len() {
            continue;
        }
        let entries_on_disk = read_le_u16(&tail[offset + 8..offset + 10]) as usize;
        let total_entries = read_le_u16(&tail[offset + 10..offset + 12]) as usize;
        let declared_entries = entries_on_disk.max(total_entries);
        if declared_entries > MAX_RESTORE_ENTRIES {
            return Err(AppError::payload_too_large(
                "backup zip contains too many entries",
            ));
        }
        return Ok(());
    }

    Err(AppError::bad_request("invalid backup zip"))
}

fn read_le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn find_cluster_prefix<R>(archive: &mut ZipArchive<R>) -> AppResult<Vec<String>>
where
    R: Read + Seek,
{
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|_| AppError::bad_request("invalid backup zip entry"))?;
        let archive_path = PathBuf::from(file.name());
        let components = safe_zip_components(&archive_path)?;
        if components.last().is_some_and(|name| name == "cluster.ini") {
            return Ok(components[..components.len() - 1].to_vec());
        }
    }
    Err(AppError::bad_request("backup zip missing cluster.ini"))
}

fn validate_restore_file_entry(
    file: &zip::read::ZipFile<'_>,
    cluster_prefix: &[String],
) -> AppResult<Option<(PathBuf, bool, u64)>> {
    if file.enclosed_name().is_none() {
        return Err(AppError::bad_request("backup zip contains unsafe path"));
    }
    if file.is_symlink() {
        return Err(AppError::bad_request("backup zip contains symlink"));
    }
    let archive_path = PathBuf::from(file.name());
    let components = safe_zip_components(&archive_path)?;
    let relative_components = strip_cluster_prefix(&components, cluster_prefix)?;
    if relative_components.is_empty() {
        return Ok(None);
    }
    Ok(Some((
        relative_components.iter().collect::<PathBuf>(),
        file.is_dir(),
        file.size(),
    )))
}

fn strip_cluster_prefix<'a>(
    components: &'a [String],
    cluster_prefix: &[String],
) -> AppResult<&'a [String]> {
    if components.len() < cluster_prefix.len() {
        return Err(AppError::bad_request(
            "backup zip entry escapes cluster root",
        ));
    }
    if components[..cluster_prefix.len()] != *cluster_prefix {
        return Err(AppError::bad_request(
            "backup zip entry escapes cluster root",
        ));
    }
    Ok(&components[cluster_prefix.len()..])
}

fn safe_zip_components(path: &Path) -> AppResult<Vec<String>> {
    let mut components = Vec::new();
    for component in path.components() {
        let Component::Normal(value) = component else {
            return Err(AppError::bad_request("backup zip contains unsafe path"));
        };
        let value = value
            .to_str()
            .ok_or_else(|| AppError::bad_request("backup zip path must be UTF-8"))?;
        let value = validate_filename(value)
            .map_err(|error| AppError::bad_request(error.to_string()))?
            .into_string();
        components.push(value);
    }
    if components.is_empty() {
        return Err(AppError::bad_request("backup zip path cannot be empty"));
    }
    Ok(components)
}

fn extract_restore_plan(
    backup_file: fs::File,
    klei_root: &Path,
    staging_name: &str,
    plan: RestorePlan,
) -> AppResult<()> {
    let mut archive =
        ZipArchive::new(backup_file).map_err(|_| AppError::bad_request("invalid backup zip"))?;
    safe_ensure_dir_under_base(klei_root, staging_name).map_err(fs_bad_request)?;
    for entry in plan.entries {
        let relative = Path::new(staging_name).join(&entry.relative_path);
        let mut file = archive
            .by_index(entry.index)
            .map_err(|_| AppError::bad_request("invalid backup zip entry"))?;
        if file.is_dir() != entry.is_dir || file.size() != entry.size {
            return Err(AppError::bad_request("backup zip changed during restore"));
        }
        if entry.is_dir {
            safe_ensure_dir_under_base(klei_root, &relative).map_err(fs_bad_request)?;
            continue;
        }
        if let Some(parent) = relative.parent()
            && !parent.as_os_str().is_empty()
        {
            safe_ensure_dir_under_base(klei_root, parent).map_err(fs_bad_request)?;
        }
        let mut output =
            safe_create_new_file_under_base(klei_root, &relative).map_err(fs_bad_request)?;
        let copied = copy_zip_entry_with_limit(&mut file, &mut output, entry.size)?;
        if copied != entry.size {
            return Err(AppError::bad_request("backup zip entry has invalid size"));
        }
    }
    Ok(())
}

fn copy_zip_entry_with_limit(
    input: &mut zip::read::ZipFile<'_>,
    output: &mut fs::File,
    expected_size: u64,
) -> AppResult<u64> {
    let copied = io::copy(&mut input.take(expected_size.saturating_add(1)), output)
        .map_err(file_error("extract restore zip entry"))?;
    if copied > expected_size {
        return Err(AppError::payload_too_large(
            "backup zip entry is larger than declared",
        ));
    }
    Ok(copied)
}

fn fs_bad_request(error: crate::infra::fs_paths::FsPathError) -> AppError {
    tracing::warn!(error = %error, "rejected unsafe backup path");
    AppError::bad_request(error.to_string())
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
