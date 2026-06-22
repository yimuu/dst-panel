//! DST lifecycle command construction for start, stop, and update routes.
//!
//! The Go backend built these operations as shell strings with `cd`, `screen`,
//! SteamCMD, and optional wrapper programs. Rust keeps the observable route
//! semantics but emits argv arrays through [`crate::infra::command::CommandRunner`],
//! so cluster and level names never pass through a shell parser.

use std::{
    io::{self, Read},
    path::Path,
    time::Duration,
};

use crate::{
    dst::{self, DstConfig},
    infra::command::{CommandError, CommandOutput, CommandRunner, CommandSpec},
    infra::fs_paths,
    infra::process::{ProcessSnapshotProvider, first_level_process},
    validation::validate_safe_command_arg,
    web::error::{AppError, AppResult},
};

use super::{
    console::{screen_command_spec, screen_session_key},
    level, mod_setup,
};

const SCREEN_PROGRAM: &str = "screen";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(60 * 20);

#[path = "clean.rs"]
mod clean;
#[path = "start_stop.rs"]
mod start_stop;
#[path = "update.rs"]
mod update;

pub(crate) use start_stop::*;
pub(crate) use update::*;

/// Validated runtime configuration needed by lifecycle commands.
#[derive(Debug, Clone)]
pub(crate) struct LifecycleContext {
    pub(crate) config: DstConfig,
    pub(crate) cluster_name: String,
}

impl LifecycleContext {
    pub(crate) fn load(root: &Path) -> AppResult<Self> {
        let config = DstConfig::load(root).map_err(file_error("load dst_config"))?;
        let cluster_name = validate_safe_command_arg("cluster name", &config.cluster)
            .map_err(|error| AppError::bad_request(error.to_string()))?
            .into_string();
        Ok(Self {
            config,
            cluster_name,
        })
    }

    pub(crate) fn level_names(&self, root: &Path) -> AppResult<Vec<String>> {
        let cluster_dir = dst::cluster_dir(root, &self.cluster_name)
            .map_err(file_error("resolve cluster directory"))?;
        let worlds = level::list_existing_worlds_from_cluster_dir(&cluster_dir)?;
        worlds
            .into_iter()
            .map(|world| {
                validate_safe_command_arg("level name", &world.uuid)
                    .map(|level| level.into_string())
                    .map_err(|error| AppError::bad_request(error.to_string()))
            })
            .collect()
    }
}

fn copy_steamclient_before_single_start_inner(root: &Path, config: &DstConfig) -> io::Result<()> {
    let src_file = Path::new(&config.steamcmd)
        .join("linux32")
        .join("steamclient.so");
    let dst_dir = Path::new(&config.force_install_dir)
        .join("bin")
        .join("lib32");
    dst::safe_ensure_configured_dir(root, &dst_dir.display().to_string())?;
    let dst_file = dst_dir.join("steamclient.so");
    let backup_file = dst_dir.join("steamclient.so.bak");

    let mut source = safe_open_existing_configured_file(root, &src_file)?;
    let mut source_bytes = Vec::new();
    source.read_to_end(&mut source_bytes)?;
    if let Some(mut existing) = safe_open_optional_configured_file(root, &dst_file)? {
        let mut existing_bytes = Vec::new();
        existing.read_to_end(&mut existing_bytes)?;
        safe_overwrite_configured_file(root, &backup_file, existing_bytes)?;
    }
    safe_overwrite_configured_file(root, &dst_file, source_bytes)?;
    Ok(())
}

fn safe_open_existing_configured_file(root: &Path, path: &Path) -> io::Result<std::fs::File> {
    if let Ok(relative) = path.strip_prefix(root) {
        fs_paths::safe_open_existing_file_under_base(root, relative).map_err(fs_path_error)
    } else {
        fs_paths::safe_open_existing_file_path(path).map_err(fs_path_error)
    }
}

fn safe_open_optional_configured_file(
    root: &Path,
    path: &Path,
) -> io::Result<Option<std::fs::File>> {
    if let Ok(relative) = path.strip_prefix(root) {
        fs_paths::safe_open_optional_existing_file_under_base(root, relative).map_err(fs_path_error)
    } else {
        fs_paths::safe_open_optional_existing_file_path(path).map_err(fs_path_error)
    }
}

fn safe_overwrite_configured_file(
    root: &Path,
    path: &Path,
    contents: impl AsRef<[u8]>,
) -> io::Result<()> {
    if let Ok(relative) = path.strip_prefix(root) {
        fs_paths::safe_overwrite_file_under_base(root, relative, contents).map_err(fs_path_error)
    } else {
        fs_paths::safe_overwrite_file_path(path, contents).map_err(fs_path_error)
    }
}

async fn run_go_strict(
    runner: &dyn CommandRunner,
    spec: CommandSpec,
    action: &'static str,
    cluster_name: &str,
    level_name: &str,
) -> AppResult<()> {
    tracing::info!(
        action,
        cluster_name,
        level_name,
        "running strict DST lifecycle command"
    );
    let output = runner.run(spec).await.map_err(command_error(action))?;
    if output.status_code != Some(0) {
        tracing::warn!(
            action,
            cluster_name,
            level_name,
            status_code = ?output.status_code,
            stdout_len = output.stdout.len(),
            stderr_len = output.stderr.len(),
            "strict DST lifecycle command exited unsuccessfully"
        );
        return Err(AppError::internal(action));
    }
    Ok(())
}

fn install_dir(config: &DstConfig) -> std::path::PathBuf {
    if config.beta == 1 {
        std::path::PathBuf::from(format!("{}-beta", config.force_install_dir))
    } else {
        std::path::PathBuf::from(&config.force_install_dir)
    }
}

async fn run_go_lenient(
    runner: &dyn CommandRunner,
    spec: CommandSpec,
    action: &'static str,
    cluster_name: &str,
    level_name: &str,
) {
    tracing::info!(
        action,
        cluster_name,
        level_name,
        "running DST lifecycle command"
    );
    match runner.run(spec).await {
        Ok(CommandOutput {
            status_code,
            stdout,
            stderr,
            ..
        }) => {
            tracing::debug!(
                action,
                cluster_name,
                level_name,
                status_code = ?status_code,
                stdout_len = stdout.len(),
                stderr_len = stderr.len(),
                "DST lifecycle command returned"
            );
        }
        Err(error) => {
            // Go swallowed many start/stop process failures after logging. Keep
            // the success envelope for these routes while preserving observability.
            tracing::warn!(
                action,
                cluster_name,
                level_name,
                error = %error,
                "DST lifecycle command failed; preserving Go success response"
            );
        }
    }
}

fn command_error(operation: &'static str) -> impl FnOnce(CommandError) -> AppError {
    move |error| {
        tracing::error!(operation, error = %error, "external command failed");
        AppError::internal(operation)
    }
}

fn fs_path_error(error: fs_paths::FsPathError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

fn file_error(operation: &'static str) -> impl FnOnce(std::io::Error) -> AppError {
    move |error| {
        tracing::error!(operation, error = %error, "DST lifecycle filesystem operation failed");
        if error.kind() == std::io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            AppError::internal(operation)
        }
    }
}
