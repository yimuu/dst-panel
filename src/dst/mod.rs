//! File-backed DST configuration helpers used by migrated routes.
//!
//! The Go backend stores most DST state in plain files under Klei's
//! `DoNotStarveTogether/<cluster>` directory. These helpers keep that layout
//! explicit and deterministic for tests by resolving the default Klei root
//! under the application root instead of the process user's real home.

use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use getrandom::fill as fill_random;
use serde::{Deserialize, Serialize};

use crate::{
    infra::fs_paths,
    validation::{validate_cluster_name, validate_level_name},
};

pub mod cluster_ini;
pub mod lua_files;
pub mod player_lists;
pub mod server_ini;

/// Contents of Go's line-oriented `dst_config` file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DstConfig {
    pub steamcmd: String,
    pub force_install_dir: String,
    pub donot_starve_server_directory: String,
    pub cluster: String,
    pub backup: String,
    pub mod_download_path: String,
    pub bin: i64,
    pub beta: i64,
    pub ugc_directory: String,
    pub persistent_storage_root: String,
    pub conf_dir: String,
}

impl DstConfig {
    /// Loads `dst_config`, applying the same important defaults as the Go code.
    pub fn load(root: &Path) -> io::Result<Self> {
        let path = root.join("dst_config");
        if !path.exists() {
            fs::write(&path, "")?;
        }

        let mut config = Self::default();
        for line in read_lines(&path)? {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim().replace("\\n", "");
            match key.trim() {
                "steamcmd" => config.steamcmd = value,
                "force_install_dir" => config.force_install_dir = value,
                "donot_starve_server_directory" => config.donot_starve_server_directory = value,
                "cluster" => config.cluster = value,
                "backup" => config.backup = value,
                "mod_download_path" => config.mod_download_path = value,
                "bin" => config.bin = value.parse().unwrap_or_default(),
                "beta" => config.beta = value.parse().unwrap_or_default(),
                "ugc_directory" => config.ugc_directory = value,
                "persistent_storage_root" => config.persistent_storage_root = value,
                "conf_dir" => config.conf_dir = value,
                _ => {}
            }
        }

        config.apply_defaults(root)?;
        Ok(config)
    }

    /// Saves `dst_config`, preserving existing non-empty path values when the
    /// request omits them, matching Go's `SaveDstConfig` compatibility behavior.
    pub fn save_with_fallbacks(mut self, root: &Path) -> io::Result<Self> {
        let old = Self::load(root).unwrap_or_default();
        if self.steamcmd.is_empty() {
            self.steamcmd = old.steamcmd;
        }
        if self.force_install_dir.is_empty() {
            self.force_install_dir = old.force_install_dir;
        }
        if self.cluster.is_empty() {
            self.cluster = old.cluster;
        }
        if self.backup.is_empty() {
            self.backup = old.backup;
        }
        if self.mod_download_path.is_empty() {
            self.mod_download_path = old.mod_download_path;
        }
        self.apply_defaults(root)?;

        write_lines(
            &root.join("dst_config"),
            &[
                format!("steamcmd={}", self.steamcmd),
                format!("force_install_dir={}", self.force_install_dir),
                format!(
                    "donot_starve_server_directory={}",
                    self.donot_starve_server_directory
                ),
                format!("ugc_directory={}", self.ugc_directory),
                format!("conf_dir={}", self.conf_dir),
                format!("persistent_storage_root={}", self.persistent_storage_root),
                format!("cluster={}", self.cluster),
                format!("backup={}", self.backup),
                format!("mod_download_path={}", self.mod_download_path),
                format!("bin={}", self.bin),
                format!("beta={}", self.beta),
            ],
        )?;
        Ok(self)
    }

    fn apply_defaults(&mut self, root: &Path) -> io::Result<()> {
        if self.cluster.is_empty() {
            self.cluster = "Cluster1".to_owned();
        }
        let cluster = validate_cluster_name(&self.cluster)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        self.cluster = cluster.as_str().to_owned();
        if self.bin == 0 {
            self.bin = 32;
        }
        if self.backup.is_empty() {
            self.backup = self.klei_root(root).join("backup").display().to_string();
        }
        if self.mod_download_path.is_empty() {
            self.mod_download_path = self
                .klei_root(root)
                .join("mod_config_download")
                .display()
                .to_string();
        }
        safe_ensure_configured_dir(root, &self.backup)?;
        safe_ensure_configured_dir(root, &self.mod_download_path)?;
        Ok(())
    }

    /// Returns the Klei `DoNotStarveTogether` directory for this config.
    pub fn klei_root(&self, root: &Path) -> PathBuf {
        let mut base = if self.persistent_storage_root.is_empty() {
            root.join(".klei").join("DoNotStarveTogether")
        } else {
            let conf_dir = if self.conf_dir.is_empty() {
                "DoNotStarveTogether"
            } else {
                &self.conf_dir
            };
            PathBuf::from(&self.persistent_storage_root).join(conf_dir)
        };
        if self.beta == 1 {
            base = PathBuf::from(format!("{}BetaBranch", base.display()));
        }
        base
    }
}

pub fn safe_ensure_configured_dir(root: &Path, directory: &str) -> io::Result<()> {
    if directory.is_empty() {
        return Ok(());
    }
    let path = Path::new(directory);
    if path.starts_with(root) {
        let relative_path = path.strip_prefix(root).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "directory path escapes root")
        })?;
        return fs_paths::safe_ensure_dir_under_base(root, relative_path).map_err(fs_path_error);
    }
    let default_klei_root = root.join(".klei").join("DoNotStarveTogether");
    if path.starts_with(&default_klei_root) {
        let relative_path = path.strip_prefix(root).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "directory path escapes root")
        })?;
        fs_paths::safe_ensure_dir_under_base(root, relative_path).map_err(fs_path_error)
    } else {
        fs_paths::safe_ensure_dir_path(path).map_err(fs_path_error)
    }
}

/// Returns the current cluster name from `dst_config`, after validation.
pub fn current_cluster_name(root: &Path) -> io::Result<String> {
    let config = DstConfig::load(root)?;
    validate_cluster_name(&config.cluster)
        .map(|cluster| cluster.as_str().to_owned())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

/// Returns the base directory for a validated cluster name.
pub fn cluster_dir(root: &Path, cluster_name: &str) -> io::Result<PathBuf> {
    let config = DstConfig::load(root)?;
    let cluster = validate_cluster_name(cluster_name)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    Ok(config.klei_root(root).join(cluster.as_str()))
}

/// Returns the base directory for the cluster selected by `dst_config`.
pub fn current_cluster_dir(root: &Path) -> io::Result<PathBuf> {
    let cluster = current_cluster_name(root)?;
    cluster_dir(root, &cluster)
}

/// Creates the Go-style baseline cluster files without starting or installing DST.
pub fn init_cluster_files(root: &Path, cluster_name: &str, token: &str) -> io::Result<()> {
    let config = DstConfig::load(root)?;
    let klei_root = config.klei_root(root);
    let cluster = validate_cluster_name(cluster_name)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let base = klei_root.join(cluster.as_str());
    if let Ok(metadata) = fs::symlink_metadata(&base) {
        if metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cluster directory must not be a symlink",
            ));
        }
        if !metadata.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cluster path must be a directory",
            ));
        }
        tracing::info!(
            cluster_name = cluster.as_str(),
            "cluster directory already exists; repairing any missing skeleton files"
        );
    } else {
        tracing::info!(
            cluster_name = cluster.as_str(),
            "initializing DST cluster file skeleton"
        );
        safe_ensure_cluster_dir(&base)?;
    }

    safe_write_cluster_file_if_missing(
        &base,
        "cluster.ini",
        cluster_ini::ClusterIni::default_for_new_cluster().to_ini(),
    )?;
    safe_write_cluster_file_if_missing(&base, "cluster_token.txt", token)?;
    write_world_files_if_missing(
        &base,
        "Master",
        "return {}",
        "return {}",
        &server_ini::ServerIni::master_default(),
    )?;
    write_world_files_if_missing(
        &base,
        "Caves",
        "return {}",
        "return {}",
        &server_ini::ServerIni::caves_default(),
    )?;
    Ok(())
}

fn write_world_files_if_missing(
    cluster_dir: &Path,
    level_name: &str,
    leveldataoverride: &str,
    modoverrides: &str,
    server_ini: &server_ini::ServerIni,
) -> io::Result<()> {
    let level = validate_level_name(level_name)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    safe_write_cluster_file_if_missing(
        cluster_dir,
        Path::new(level.as_str()).join("leveldataoverride.lua"),
        leveldataoverride,
    )?;
    safe_write_cluster_file_if_missing(
        cluster_dir,
        Path::new(level.as_str()).join("modoverrides.lua"),
        modoverrides,
    )?;
    safe_write_cluster_file_if_missing(
        cluster_dir,
        Path::new(level.as_str()).join("server.ini"),
        server_ini.to_ini(),
    )?;
    Ok(())
}

/// Writes one level directory and its core config files.
pub fn write_world_files(
    cluster_dir: &Path,
    level_name: &str,
    leveldataoverride: &str,
    modoverrides: &str,
    server_ini: &server_ini::ServerIni,
) -> io::Result<()> {
    let level = validate_level_name(level_name)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    safe_write_cluster_file(
        cluster_dir,
        Path::new(level.as_str()).join("leveldataoverride.lua"),
        leveldataoverride,
    )?;
    safe_write_cluster_file(
        cluster_dir,
        Path::new(level.as_str()).join("modoverrides.lua"),
        modoverrides,
    )?;
    safe_write_cluster_file(
        cluster_dir,
        Path::new(level.as_str()).join("server.ini"),
        server_ini.to_ini(),
    )?;
    Ok(())
}

/// Ensures a cluster directory exists without following a symlinked cluster
/// path under the Klei root.
pub fn safe_ensure_cluster_dir(cluster_dir: &Path) -> io::Result<()> {
    let (base, cluster_path) = cluster_base_relative_path(cluster_dir, Path::new(""))?;
    fs_paths::safe_ensure_dir_under_base(base, cluster_path).map_err(fs_path_error)
}

/// Reads a file relative to a cluster directory without following symlinks in
/// the cluster name, intermediate level directories, or the leaf.
pub fn safe_read_cluster_file_to_string(
    cluster_dir: &Path,
    relative_path: impl AsRef<Path>,
) -> io::Result<Option<String>> {
    let (base, relative_path) = cluster_base_relative_path(cluster_dir, relative_path.as_ref())?;
    let Some(mut file) = fs_paths::safe_open_optional_existing_file_under_base(base, relative_path)
        .map_err(fs_path_error)?
    else {
        return Ok(None);
    };
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(Some(contents))
}

/// Opens an optional file relative to a cluster directory without following
/// symlinks in the cluster name, intermediate level directories, or the leaf.
///
/// Download routes use this instead of resolving a string path and reopening it,
/// keeping authorization and file access tied to the same descriptor traversal.
pub fn safe_open_cluster_file(
    cluster_dir: &Path,
    relative_path: impl AsRef<Path>,
) -> io::Result<Option<fs::File>> {
    let (base, relative_path) = cluster_base_relative_path(cluster_dir, relative_path.as_ref())?;
    fs_paths::safe_open_optional_existing_file_under_base(base, relative_path)
        .map_err(fs_path_error)
}

/// Overwrites a file relative to a cluster directory without following
/// symlinks in the cluster name, intermediate level directories, or the leaf.
pub fn safe_write_cluster_file(
    cluster_dir: &Path,
    relative_path: impl AsRef<Path>,
    contents: impl AsRef<[u8]>,
) -> io::Result<()> {
    let (base, relative_path) = cluster_base_relative_path(cluster_dir, relative_path.as_ref())?;
    if let Some(parent) = relative_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs_paths::safe_ensure_dir_under_base(&base, parent).map_err(fs_path_error)?;
    }
    fs_paths::safe_overwrite_file_under_base(base, relative_path, contents).map_err(fs_path_error)
}

/// Removes a directory relative to a cluster without following symlinked
/// cluster, level, or child directory entries.
pub fn safe_remove_cluster_dir(
    cluster_dir: &Path,
    relative_path: impl AsRef<Path>,
) -> io::Result<()> {
    let (base, relative_path) = cluster_base_relative_path(cluster_dir, relative_path.as_ref())?;
    if !fs_paths::safe_directory_exists_under_base(&base, &relative_path).map_err(fs_path_error)? {
        return Ok(());
    }
    fs_paths::safe_remove_dir_all_under_base(base, relative_path).map_err(fs_path_error)
}

/// Removes a regular file relative to a cluster without following symlinked
/// cluster, level, or file entries. Missing Klei roots, missing parents, and
/// missing leaves are reported as `Ok(false)` so cleanup routes can stay
/// idempotent while still rejecting unsafe paths.
pub fn safe_remove_cluster_file(
    cluster_dir: &Path,
    relative_path: impl AsRef<Path>,
) -> io::Result<bool> {
    let (base, relative_path) = cluster_base_relative_path(cluster_dir, relative_path.as_ref())?;
    fs_paths::safe_remove_file_under_base(base, relative_path).map_err(fs_path_error)
}

/// Checks whether a directory relative to a cluster exists without accepting a
/// symlink at the cluster, level, or child directory boundary.
pub fn safe_cluster_dir_exists(
    cluster_dir: &Path,
    relative_path: impl AsRef<Path>,
) -> io::Result<bool> {
    let (base, relative_path) = cluster_base_relative_path(cluster_dir, relative_path.as_ref())?;
    fs_paths::safe_directory_exists_under_base(base, relative_path).map_err(fs_path_error)
}

/// Checks a cluster-relative directory, treating a missing Klei root as absent.
///
/// Read-only routes use this to inspect optional DST state without creating the
/// Klei root or converting a not-yet-initialized install into a bad request.
pub fn safe_optional_cluster_dir_exists(
    cluster_dir: &Path,
    relative_path: impl AsRef<Path>,
) -> io::Result<bool> {
    let (base, relative_path) = cluster_base_relative_path(cluster_dir, relative_path.as_ref())?;
    fs_paths::safe_directory_exists_under_base(base, relative_path).map_err(fs_path_error)
}

/// Renames a directory relative to a cluster using the same no-follow traversal
/// as deletion. Callers use this for staged destructive operations where a
/// metadata update may need to be rolled back before final removal.
pub fn safe_rename_cluster_dir(
    cluster_dir: &Path,
    from_path: impl AsRef<Path>,
    to_path: impl AsRef<Path>,
) -> io::Result<()> {
    let (base, from_path) = cluster_base_relative_path(cluster_dir, from_path.as_ref())?;
    let (_, to_path) = cluster_base_relative_path(cluster_dir, to_path.as_ref())?;
    fs_paths::safe_rename_dir_under_base(base, from_path, to_path).map_err(fs_path_error)
}

fn safe_write_cluster_file_if_missing(
    cluster_dir: &Path,
    relative_path: impl AsRef<Path>,
    contents: impl AsRef<[u8]>,
) -> io::Result<()> {
    if safe_read_cluster_file_to_string(cluster_dir, relative_path.as_ref())?.is_some() {
        return Ok(());
    }
    safe_write_cluster_file(cluster_dir, relative_path, contents)
}

fn cluster_base_relative_path(
    cluster_dir: &Path,
    relative_path: &Path,
) -> io::Result<(PathBuf, PathBuf)> {
    let (klei_root, cluster_name) = cluster_base_parts(cluster_dir)?;
    let (base, klei_relative_path) = klei_root_base_and_relative(&klei_root)?;
    let mut cluster_relative_path = klei_relative_path.join(cluster_name);
    if !relative_path.as_os_str().is_empty() {
        cluster_relative_path.push(relative_path);
    }
    Ok((base.to_path_buf(), cluster_relative_path))
}

fn klei_root_base_and_relative(klei_root: &Path) -> io::Result<(&Path, PathBuf)> {
    let leaf = klei_root
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing Klei root name"))?;
    let parent = klei_root
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing Klei root parent"))?;

    if parent.file_name().and_then(|value| value.to_str()) == Some(".klei") {
        let app_root = parent.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "missing application root")
        })?;
        reject_symlinked_klei_anchor(app_root)?;
        return Ok((app_root, Path::new(".klei").join(leaf)));
    }

    reject_symlinked_klei_anchor(parent)?;
    Ok((parent, PathBuf::from(leaf)))
}

fn reject_symlinked_klei_anchor(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Klei root anchor must not be a symlink",
        ));
    }
    Ok(())
}

/// Generates a version-4 UUID using the already-present `getrandom` dependency.
pub fn generate_uuid_v4() -> io::Result<String> {
    let mut bytes = [0_u8; 16];
    fill_random(&mut bytes).map_err(|error| io::Error::other(error.to_string()))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        u16::from_be_bytes([bytes[4], bytes[5]]),
        u16::from_be_bytes([bytes[6], bytes[7]]),
        u16::from_be_bytes([bytes[8], bytes[9]]),
        u64::from_be_bytes([
            0, 0, bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ])
    ))
}

fn read_lines(path: &Path) -> io::Result<Vec<String>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .map(ToOwned::to_owned)
        .collect())
}

pub(crate) fn write_lines(path: &Path, lines: &[String]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut contents = lines.join("\n");
    if !contents.is_empty() {
        contents.push('\n');
    }
    fs::write(path, contents)
}

fn cluster_base_parts(cluster_dir: &Path) -> io::Result<(PathBuf, String)> {
    let cluster_name = cluster_dir
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing cluster name"))?;
    let cluster = validate_cluster_name(cluster_name)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let klei_root = cluster_dir
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing Klei root"))?;
    Ok((klei_root.to_path_buf(), cluster.as_str().to_owned()))
}

fn fs_path_error(error: fs_paths::FsPathError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

pub(crate) fn parse_bool(value: Option<&String>, default: bool) -> bool {
    value
        .and_then(|value| value.trim().parse::<bool>().ok())
        .unwrap_or(default)
}

pub(crate) fn parse_u64(value: Option<&String>, default: u64) -> u64 {
    value
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(default)
}
