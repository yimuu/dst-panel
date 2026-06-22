//! Preinstall cluster template application.
//!
//! Go replaces the active cluster directory with a directory from
//! `static/preinstall/<name>`, then copies identity files from the old cluster
//! back into the new one. This module preserves that file contract while using
//! staged rename and no-follow path helpers so failures can roll back.

use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use crate::{
    dst::DstConfig,
    infra::fs_paths::{
        safe_directory_exists_under_base, safe_ensure_dir_under_base,
        safe_open_existing_file_under_base, safe_open_optional_existing_file_under_base,
        safe_overwrite_file_under_base, safe_remove_dir_all_under_base, safe_rename_dir_under_base,
        safe_resolve_under_base,
    },
    validation::{validate_cluster_name, validate_filename},
    web::error::{AppError, AppResult},
};

const STAGING_DIR: &str = ".dst-admin-rust-preinstall-staging";
const IDENTITY_FILES: &[&str] = &[
    "adminlist.txt",
    "blocklist.txt",
    "cluster_token.txt",
    "whitelist.txt",
];

/// Applies a named preinstall template to the current cluster.
pub(crate) fn apply(root: &Path, name: &str) -> AppResult<()> {
    let template_name = validate_filename(name)
        .map_err(|error| AppError::bad_request(error.to_string()))?
        .into_string();
    let config = DstConfig::load(root).map_err(file_error("load dst_config"))?;
    let cluster_name = validate_cluster_name(&config.cluster)
        .map_err(|error| AppError::bad_request(error.to_string()))?
        .into_string();
    if cluster_name == STAGING_DIR {
        return Err(AppError::bad_request(
            "cluster name conflicts with reserved preinstall staging path",
        ));
    }
    let klei_root = config.klei_root(root);
    let template_base = root.join("static").join("preinstall");
    let template_dir =
        safe_resolve_under_base(&template_base, &template_name).map_err(fs_bad_request)?;
    if !safe_directory_exists_under_base(&template_base, &template_name).map_err(fs_bad_request)? {
        return Err(AppError::bad_request("preinstall template does not exist"));
    }

    if safe_directory_exists_under_base(&klei_root, STAGING_DIR).map_err(fs_bad_request)? {
        return Err(AppError::bad_request(
            "preinstall staging directory already exists",
        ));
    }
    let had_existing =
        safe_directory_exists_under_base(&klei_root, &cluster_name).map_err(fs_bad_request)?;
    if had_existing {
        safe_rename_dir_under_base(&klei_root, &cluster_name, STAGING_DIR)
            .map_err(fs_bad_request)?;
    }

    let copy_result = copy_template_tree(&template_dir, &klei_root, &cluster_name)
        .and_then(|_| restore_identity_files(&klei_root, STAGING_DIR, &cluster_name));
    if let Err(error) = copy_result {
        tracing::error!(
            cluster_name,
            template_name,
            error = %error,
            "preinstall template application failed; attempting rollback"
        );
        rollback_preinstall(&klei_root, &cluster_name, had_existing);
        return Err(file_error("apply preinstall template")(error));
    }

    if had_existing {
        safe_remove_dir_all_under_base(&klei_root, STAGING_DIR).map_err(fs_bad_request)?;
    }
    tracing::info!(
        cluster_name,
        template_name,
        "applied DST preinstall template"
    );
    Ok(())
}

fn copy_template_tree(template_dir: &Path, klei_root: &Path, cluster_name: &str) -> io::Result<()> {
    safe_ensure_dir_under_base(klei_root, cluster_name).map_err(fs_path_error)?;
    copy_template_entries(template_dir, template_dir, klei_root, cluster_name)
}

fn copy_template_entries(
    template_root: &Path,
    current_dir: &Path,
    klei_root: &Path,
    cluster_name: &str,
) -> io::Result<()> {
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "preinstall template must not contain symlinks",
            ));
        }
        let relative = path.strip_prefix(template_root).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "preinstall template path escapes root",
            )
        })?;
        let destination = PathBuf::from(cluster_name).join(relative);
        if metadata.is_dir() {
            safe_ensure_dir_under_base(klei_root, &destination).map_err(fs_path_error)?;
            copy_template_entries(template_root, &path, klei_root, cluster_name)?;
        } else if metadata.is_file() {
            let mut file = safe_open_existing_file_under_base(template_root, relative)
                .map_err(fs_path_error)?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            if let Some(parent) = destination.parent()
                && !parent.as_os_str().is_empty()
            {
                safe_ensure_dir_under_base(klei_root, parent).map_err(fs_path_error)?;
            }
            safe_overwrite_file_under_base(klei_root, destination, bytes).map_err(fs_path_error)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "preinstall template contains unsupported file type",
            ));
        }
    }
    Ok(())
}

fn restore_identity_files(
    klei_root: &Path,
    backup_dir: &str,
    cluster_name: &str,
) -> io::Result<()> {
    for filename in IDENTITY_FILES {
        let source = PathBuf::from(backup_dir).join(filename);
        let Some(mut file) = safe_open_optional_existing_file_under_base(klei_root, &source)
            .map_err(fs_path_error)?
        else {
            continue;
        };
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let destination = PathBuf::from(cluster_name).join(filename);
        safe_overwrite_file_under_base(klei_root, destination, bytes).map_err(fs_path_error)?;
    }
    Ok(())
}

fn rollback_preinstall(klei_root: &Path, cluster_name: &str, had_existing: bool) {
    if let Ok(true) = safe_directory_exists_under_base(klei_root, cluster_name)
        && let Err(error) = safe_remove_dir_all_under_base(klei_root, cluster_name)
    {
        tracing::error!(cluster_name, error = %error, "failed to remove partial preinstall cluster");
    }
    if had_existing
        && let Ok(true) = safe_directory_exists_under_base(klei_root, STAGING_DIR)
        && let Err(error) = safe_rename_dir_under_base(klei_root, STAGING_DIR, cluster_name)
    {
        tracing::error!(cluster_name, error = %error, "failed to restore preinstall backup");
    }
}

fn fs_path_error(error: crate::infra::fs_paths::FsPathError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}

fn fs_bad_request(error: crate::infra::fs_paths::FsPathError) -> AppError {
    tracing::warn!(error = %error, "rejected unsafe preinstall path");
    AppError::bad_request(error.to_string())
}

fn file_error(operation: &'static str) -> impl FnOnce(io::Error) -> AppError {
    move |error| {
        tracing::error!(operation, error = %error, "preinstall filesystem operation failed");
        if error.kind() == io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            AppError::internal(operation)
        }
    }
}
