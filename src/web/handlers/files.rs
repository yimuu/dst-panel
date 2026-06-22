//! File upload/download handlers for UGC mods and the frontend background.

use std::{
    io::Read,
    path::{Path, PathBuf},
};

use axum::{
    Json,
    body::Body,
    extract::{Multipart, State},
    http::{
        StatusCode,
        header::{CONTENT_LENGTH, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use bytes::Bytes;

use crate::{
    dst::{DstConfig, safe_ensure_configured_dir},
    infra::fs_paths::{
        safe_ensure_dir_under_base, safe_open_optional_existing_file_under_base,
        safe_overwrite_file_under_base,
    },
    validation::validate_filename,
    web::app::AppState,
    web::error::{AppError, AppResult},
    web::handlers::legacy_empty_success,
};

const MAX_UPLOAD_FILE_BYTES: usize = 20 * 1024 * 1024;

/// Handles Go-compatible UGC mod multipart uploads.
pub(crate) async fn upload_ugc_handler(
    State(state): State<AppState>,
    multipart: Multipart,
) -> AppResult<Json<crate::web::response::LoginResponse<serde_json::Value>>> {
    let parsed = collect_ugc_upload(multipart).await?;
    if parsed.files.len() != parsed.file_paths.len() {
        return Err(AppError::bad_request(
            "files and filePaths counts must match",
        ));
    }

    let config = DstConfig::load(&state.root_path)?;
    let base = ugc_upload_root(&config);
    ensure_absolute_dir(&state.root_path, &base)?;

    for (index, upload) in parsed.files.into_iter().enumerate() {
        let relative = upload_relative_path(&parsed.file_paths[index], &upload.filename)?;
        if let Some(parent) = relative.parent()
            && !parent.as_os_str().is_empty()
        {
            safe_ensure_dir_under_base(&base, parent).map_err(fs_bad_request)?;
        }
        safe_overwrite_file_under_base(&base, &relative, &upload.contents)
            .map_err(fs_bad_request)?;
        tracing::info!(
            filename = %upload.filename,
            bytes = upload.contents.len(),
            "stored UGC upload file"
        );
    }

    Ok(Json(legacy_empty_success()))
}

/// Uploads the custom panel background image.
pub(crate) async fn upload_background_handler(
    State(state): State<AppState>,
    multipart: Multipart,
) -> AppResult<Json<crate::web::response::LoginResponse<serde_json::Value>>> {
    let files = collect_background_upload(multipart).await?;

    safe_ensure_dir_under_base(&state.root_path, "dist/assets").map_err(fs_bad_request)?;
    for file in files {
        safe_overwrite_file_under_base(
            &state.root_path,
            "dist/assets/background.png",
            &file.contents,
        )
        .map_err(fs_bad_request)?;
        tracing::info!(
            filename = %file.filename,
            bytes = file.contents.len(),
            "stored panel background"
        );
    }
    Ok(Json(legacy_empty_success()))
}

/// Serves the custom background as a raw PNG file.
pub(crate) async fn get_background_handler(State(state): State<AppState>) -> AppResult<Response> {
    let Some(mut file) =
        safe_open_optional_existing_file_under_base(&state.root_path, "dist/assets/background.png")
            .map_err(fs_bad_request)?
    else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let metadata = file
        .metadata()
        .map_err(|_| AppError::internal("failed to read background metadata"))?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    tracing::info!(bytes = contents.len(), "served panel background");
    Ok((
        [
            (CONTENT_TYPE, "image/png".to_owned()),
            (CONTENT_LENGTH, metadata.len().to_string()),
        ],
        Body::from(contents),
    )
        .into_response())
}

async fn collect_ugc_upload(mut multipart: Multipart) -> AppResult<UgcUpload> {
    let mut upload = UgcUpload::default();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::bad_request("invalid multipart upload"))?
    {
        let Some(name) = field.name().map(str::to_owned) else {
            continue;
        };
        match name.as_str() {
            "filePaths" => {
                let value = field
                    .text()
                    .await
                    .map_err(|_| AppError::bad_request("invalid filePaths field"))?;
                upload.file_paths.push(value);
            }
            "files" => upload.files.push(read_upload_file(field).await?),
            _ => {}
        }
    }
    Ok(upload)
}

async fn collect_background_upload(mut multipart: Multipart) -> AppResult<Vec<UploadedFile>> {
    let mut files = Vec::new();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::bad_request("invalid multipart upload"))?
    {
        if field.name() == Some("file") {
            files.push(read_upload_file(field).await?);
        }
    }
    Ok(files)
}

async fn read_upload_file(field: axum::extract::multipart::Field<'_>) -> AppResult<UploadedFile> {
    let filename = field
        .file_name()
        .map(str::to_owned)
        .ok_or_else(|| AppError::bad_request("uploaded file is missing filename"))?;
    let filename = validate_filename(&filename)
        .map_err(|error| AppError::bad_request(error.to_string()))?
        .into_string();
    let contents = field
        .bytes()
        .await
        .map_err(|_| AppError::bad_request("invalid uploaded file"))?;
    if contents.len() > MAX_UPLOAD_FILE_BYTES {
        return Err(AppError::payload_too_large("uploaded file is too large"));
    }
    Ok(UploadedFile { filename, contents })
}

fn ugc_upload_root(config: &DstConfig) -> PathBuf {
    if config.ugc_directory.is_empty() {
        PathBuf::from(&config.force_install_dir)
            .join("ugc_mods")
            .join(&config.cluster)
            .join("Master")
            .join("content")
            .join("322330")
    } else {
        PathBuf::from(&config.ugc_directory)
            .join("content")
            .join("322330")
    }
}

fn upload_relative_path(file_path: &str, filename: &str) -> AppResult<PathBuf> {
    let directories = safe_upload_parent_components(file_path)?;
    let mut relative = PathBuf::new();
    for directory in directories {
        relative.push(directory);
    }
    relative.push(filename);
    Ok(relative)
}

fn safe_upload_parent_components(file_path: &str) -> AppResult<Vec<String>> {
    if file_path.is_empty() || file_path.contains('\\') || file_path.contains('\0') {
        return Err(AppError::bad_request("invalid upload path"));
    }
    let mut components = file_path.split('/').collect::<Vec<_>>();
    if components.iter().any(|part| part.is_empty()) {
        return Err(AppError::bad_request("invalid upload path"));
    }
    components.pop();
    let mut safe = Vec::new();
    for component in components {
        let component = validate_filename(component)
            .map_err(|error| AppError::bad_request(error.to_string()))?
            .into_string();
        safe.push(component);
    }
    Ok(safe)
}

fn ensure_absolute_dir(root: &Path, directory: &Path) -> AppResult<()> {
    safe_ensure_configured_dir(root, &directory.display().to_string())?;
    Ok(())
}

fn fs_bad_request(error: impl std::fmt::Display) -> AppError {
    AppError::bad_request(error.to_string())
}

#[derive(Debug, Default)]
struct UgcUpload {
    files: Vec<UploadedFile>,
    file_paths: Vec<String>,
}

#[derive(Debug)]
struct UploadedFile {
    filename: String,
    contents: Bytes,
}
