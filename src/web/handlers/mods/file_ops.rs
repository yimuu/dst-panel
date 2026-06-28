//! Filesystem, SteamCMD, and UGC helpers for mod routes.

use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{
    dst::{DstConfig, safe_ensure_configured_dir},
    infra::{
        command::CommandSpec,
        fs_paths::{
            safe_directory_exists_under_base, safe_directory_path_exists,
            safe_open_optional_existing_file_under_base, safe_remove_dir_all_under_base,
            safe_rename_dir_under_base,
        },
        http_client::HttpRequest,
    },
    validation::{validate_filename, validate_mod_id},
    web::{
        app::AppState,
        error::{AppError, AppResult},
    },
};

use super::{lua_config::mod_config_from_lua_source, steam_api::SteamDetail};

const TRUSTED_FILE_URL_HOSTS: &[&str] = &[
    "cdn.steamusercontent.com",
    "steamusercontent-a.akamaihd.net",
    "steamuserimages-a.akamaihd.net",
    "steamcdn-a.akamaihd.net",
    "cdn.cloudflare.steamstatic.com",
    "shared.cloudflare.steamstatic.com",
];
pub(super) const MAX_MODINFO_BYTES: usize = 1024 * 1024;
pub(super) const STEAMCMD_MOD_DOWNLOAD_FAILED: &str = "steamcmd mod download failed";
const STEAMCMD_MOD_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const STEAMCMD_MOD_DOWNLOAD_OUTPUT_LIMIT: usize = 64 * 1024;
pub(super) fn mod_config_from_existing_files(
    root: &Path,
    config: &DstConfig,
    lang: &str,
    mod_id: &str,
) -> AppResult<String> {
    Ok(
        find_mod_config_from_existing_files(root, config, lang, mod_id)?
            .unwrap_or_else(|| "{}".to_owned()),
    )
}

pub(super) async fn mod_config_from_existing_or_steamcmd(
    state: &AppState,
    config: &DstConfig,
    lang: &str,
    mod_id: &str,
) -> AppResult<String> {
    if let Some(mod_config) =
        find_mod_config_from_existing_files(&state.root_path, config, lang, mod_id)?
    {
        return Ok(mod_config);
    }

    download_mod_with_steamcmd(state, config, mod_id).await?;
    let mod_config = find_mod_config_from_existing_files(&state.root_path, config, lang, mod_id)?
        .unwrap_or_else(|| {
            tracing::warn!(
                modid = %mod_id,
                "SteamCMD finished but downloaded modinfo.lua was not found"
            );
            "{}".to_owned()
        });
    Ok(mod_config)
}

pub(super) async fn mod_config_from_detail_source(
    state: &AppState,
    config: &DstConfig,
    lang: &str,
    mod_id: &str,
    detail: &SteamDetail,
) -> AppResult<String> {
    if let Some(file_url) = trusted_file_url(detail, mod_id) {
        return mod_config_from_file_url_zip(state, lang, mod_id, file_url).await;
    }
    mod_config_from_existing_or_steamcmd(state, config, lang, mod_id).await
}

pub(super) async fn refresh_mod_config_for_detail(
    state: &AppState,
    config: &DstConfig,
    lang: &str,
    mod_id: &str,
    detail: &SteamDetail,
) -> AppResult<String> {
    if let Some(file_url) = trusted_file_url(detail, mod_id) {
        return mod_config_from_file_url_zip(state, lang, mod_id, file_url).await;
    }
    refresh_mod_config_from_steamcmd(state, config, lang, mod_id).await
}

fn trusted_file_url<'a>(detail: &'a SteamDetail, mod_id: &str) -> Option<&'a str> {
    let file_url = detail
        .file_url
        .as_deref()
        .filter(|value| !value.is_empty())?;
    let Ok(url) = reqwest::Url::parse(file_url) else {
        tracing::warn!(modid = %mod_id, "ignored malformed Steam file_url");
        return None;
    };
    if url.scheme() != "https" {
        tracing::warn!(modid = %mod_id, scheme = %url.scheme(), "ignored untrusted Steam file_url scheme");
        return None;
    }
    let Some(host) = url.host_str() else {
        tracing::warn!(modid = %mod_id, "ignored Steam file_url without host");
        return None;
    };
    let host = host.to_ascii_lowercase();
    if TRUSTED_FILE_URL_HOSTS
        .iter()
        .any(|allowed| host == *allowed)
    {
        return Some(file_url);
    }
    tracing::warn!(modid = %mod_id, host = %host, "ignored untrusted Steam file_url host");
    None
}

async fn mod_config_from_file_url_zip(
    state: &AppState,
    lang: &str,
    mod_id: &str,
    file_url: &str,
) -> AppResult<String> {
    let response = state
        .http_client
        .send(HttpRequest::new("GET", file_url.to_owned()))
        .await
        .map_err(|error| {
            tracing::warn!(modid = %mod_id, error = %error, "v1 mod zip download failed");
            AppError::bad_request("v1 mod zip download failed")
        })?;
    if response.status != 200 {
        tracing::warn!(
            modid = %mod_id,
            status = response.status,
            "v1 mod zip download returned non-200"
        );
        return Err(AppError::bad_request("v1 mod zip download failed"));
    }
    let script = modinfo_lua_from_zip(&response.body)?;
    tracing::info!(
        modid = %mod_id,
        bytes = response.body.len(),
        "parsed v1 modinfo.lua from file_url zip"
    );
    Ok(mod_config_from_lua_source(&script, lang, mod_id))
}

fn modinfo_lua_from_zip(body: &[u8]) -> AppResult<String> {
    let cursor = Cursor::new(body);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|_| AppError::bad_request("v1 mod zip is malformed"))?;
    let mut file = archive
        .by_name("modinfo.lua")
        .map_err(|_| AppError::bad_request("v1 mod zip missing modinfo.lua"))?;
    if file.size() > MAX_MODINFO_BYTES as u64 {
        return Err(AppError::payload_too_large("modinfo.lua is too large"));
    }
    let mut contents = Vec::new();
    file.by_ref()
        .take(MAX_MODINFO_BYTES as u64 + 1)
        .read_to_end(&mut contents)?;
    if contents.len() > MAX_MODINFO_BYTES {
        return Err(AppError::payload_too_large("modinfo.lua is too large"));
    }
    String::from_utf8(contents).map_err(|_| AppError::bad_request("modinfo.lua must be UTF-8"))
}

async fn refresh_mod_config_from_steamcmd(
    state: &AppState,
    config: &DstConfig,
    lang: &str,
    mod_id: &str,
) -> AppResult<String> {
    let staged = stage_downloaded_mod_dir(&state.root_path, config, mod_id)?;
    let result = async {
        download_mod_with_steamcmd(state, config, mod_id).await?;
        let Some(mod_config) =
            find_mod_config_from_existing_files(&state.root_path, config, lang, mod_id)?
        else {
            tracing::warn!(
                modid = %mod_id,
                "SteamCMD refresh finished but downloaded modinfo.lua was not found"
            );
            return Err(AppError::bad_request("downloaded modinfo.lua not found"));
        };
        Ok(mod_config)
    }
    .await;

    match result {
        Ok(mod_config) => {
            if let Some(staged) = staged {
                staged.commit()?;
            }
            Ok(mod_config)
        }
        Err(error) => {
            if let Some(staged) = staged {
                staged.rollback();
            }
            Err(error)
        }
    }
}

fn find_mod_config_from_existing_files(
    root: &Path,
    config: &DstConfig,
    lang: &str,
    mod_id: &str,
) -> AppResult<Option<String>> {
    for base in modinfo_candidate_bases(config, mod_id) {
        if !configured_directory_exists(root, &base)? {
            continue;
        }
        let relative = PathBuf::from(mod_id).join("modinfo.lua");
        let Some(mut file) = safe_open_optional_existing_file_under_base(&base, &relative)
            .map_err(fs_bad_request)?
        else {
            continue;
        };
        let mut contents = Vec::new();
        file.by_ref()
            .take(MAX_MODINFO_BYTES as u64 + 1)
            .read_to_end(&mut contents)?;
        if contents.len() > MAX_MODINFO_BYTES {
            return Err(AppError::payload_too_large("modinfo.lua is too large"));
        }
        let script = String::from_utf8(contents)
            .map_err(|_| AppError::bad_request("modinfo.lua must be UTF-8"))?;
        tracing::info!(modid = %mod_id, "parsed existing modinfo.lua");
        return Ok(Some(mod_config_from_lua_source(&script, lang, mod_id)));
    }

    Ok(None)
}

async fn download_mod_with_steamcmd(
    state: &AppState,
    config: &DstConfig,
    mod_id: &str,
) -> AppResult<()> {
    safe_ensure_configured_dir(&state.root_path, &config.mod_download_path)?;
    let program = steamcmd_program(config);
    let spec = CommandSpec::new(program.display().to_string())
        // SteamCMD accepts commands as argv tokens. Keeping every user-derived
        // value in a separate argument preserves shell-free execution.
        .extend_args([
            "+login",
            "anonymous",
            "+force_install_dir",
            config.mod_download_path.as_str(),
            "+workshop_download_item",
            "322330",
            mod_id,
            "+quit",
        ])
        .with_timeout(STEAMCMD_MOD_DOWNLOAD_TIMEOUT)
        .with_output_limit(STEAMCMD_MOD_DOWNLOAD_OUTPUT_LIMIT);
    tracing::info!(modid = %mod_id, program = %program.display(), "downloading workshop mod with SteamCMD");
    let output = state.command_runner.run(spec).await.map_err(|error| {
        tracing::warn!(modid = %mod_id, error = %error, "SteamCMD mod download failed");
        AppError::bad_request(STEAMCMD_MOD_DOWNLOAD_FAILED)
    })?;
    if output.timed_out || output.status_code != Some(0) {
        tracing::warn!(
            modid = %mod_id,
            status_code = ?output.status_code,
            timed_out = output.timed_out,
            "SteamCMD mod download exited unsuccessfully"
        );
        return Err(AppError::bad_request(STEAMCMD_MOD_DOWNLOAD_FAILED));
    }
    tracing::info!(
        modid = %mod_id,
        stdout_len = output.stdout.len(),
        stderr_len = output.stderr.len(),
        "SteamCMD mod download finished"
    );
    Ok(())
}

fn stage_downloaded_mod_dir(
    root: &Path,
    config: &DstConfig,
    mod_id: &str,
) -> AppResult<Option<StagedModDownload>> {
    let base = mod_download_content_base(config);
    if !configured_directory_exists(root, &base)? {
        return Ok(None);
    }
    if !safe_directory_exists_under_base(&base, mod_id).map_err(fs_bad_request)? {
        return Ok(None);
    }
    let backup_name = format!("{mod_id}.refresh.{}", monotonic_backup_suffix());
    safe_rename_dir_under_base(&base, mod_id, &backup_name).map_err(fs_bad_request)?;
    tracing::info!(
        modid = %mod_id,
        backup = %backup_name,
        "staged existing SteamCMD mod cache before refresh"
    );
    Ok(Some(StagedModDownload {
        base,
        mod_id: mod_id.to_owned(),
        backup_name,
    }))
}

fn monotonic_backup_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

struct StagedModDownload {
    base: PathBuf,
    mod_id: String,
    backup_name: String,
}

impl StagedModDownload {
    fn commit(self) -> AppResult<()> {
        if safe_directory_exists_under_base(&self.base, &self.backup_name)
            .map_err(fs_bad_request)?
        {
            safe_remove_dir_all_under_base(&self.base, &self.backup_name)
                .map_err(fs_bad_request)?;
        }
        tracing::info!(
            modid = %self.mod_id,
            backup = %self.backup_name,
            "removed staged SteamCMD mod cache backup"
        );
        Ok(())
    }

    fn rollback(self) {
        if safe_directory_exists_under_base(&self.base, &self.mod_id).unwrap_or(false)
            && let Err(error) = safe_remove_dir_all_under_base(&self.base, &self.mod_id)
        {
            tracing::warn!(
                modid = %self.mod_id,
                error = %error,
                "failed to remove partial refreshed SteamCMD mod cache during rollback"
            );
            return;
        }
        if let Err(error) = safe_rename_dir_under_base(&self.base, &self.backup_name, &self.mod_id)
        {
            tracing::warn!(
                modid = %self.mod_id,
                backup = %self.backup_name,
                error = %error,
                "failed to restore staged SteamCMD mod cache after refresh error"
            );
            return;
        }
        tracing::info!(
            modid = %self.mod_id,
            backup = %self.backup_name,
            "restored staged SteamCMD mod cache after refresh error"
        );
    }
}

fn steamcmd_program(config: &DstConfig) -> PathBuf {
    let steamcmd_dir = PathBuf::from(&config.steamcmd);
    if config.steamcmd.is_empty() {
        return PathBuf::from("steamcmd");
    }
    if cfg!(windows) {
        return steamcmd_dir.join("steamcmd.exe");
    }
    let unix_program = steamcmd_dir.join("steamcmd");
    if unix_program.exists() {
        unix_program
    } else if steamcmd_dir.join("linux64").join("steamcmd").exists() {
        steamcmd_dir.join("linux64").join("steamcmd")
    } else {
        steamcmd_dir.join("steamcmd.sh")
    }
}

fn modinfo_candidate_bases(config: &DstConfig, mod_id: &str) -> Vec<PathBuf> {
    let mut bases = Vec::new();
    if config.ugc_directory.is_empty() {
        for level in ["Master", "Caves"] {
            bases.push(
                PathBuf::from(&config.force_install_dir)
                    .join("ugc_mods")
                    .join(&config.cluster)
                    .join(level)
                    .join("content")
                    .join("322330"),
            );
        }
    } else {
        bases.push(
            PathBuf::from(&config.ugc_directory)
                .join("content")
                .join("322330"),
        );
    }
    bases.push(
        PathBuf::from(&config.mod_download_path)
            .join("steamapps")
            .join("workshop")
            .join("content")
            .join("322330"),
    );
    tracing::debug!(modid = %mod_id, candidate_count = bases.len(), "built modinfo candidates");
    bases
}
pub(super) fn delete_mod_download_dir(root: &Path, mod_id: &str) -> AppResult<()> {
    let config = DstConfig::load(root)?;
    let base = mod_download_content_base(&config);
    if !configured_directory_exists(root, &base)? {
        return Ok(());
    }
    if safe_directory_exists_under_base(&base, mod_id).map_err(fs_bad_request)? {
        safe_remove_dir_all_under_base(&base, mod_id).map_err(fs_bad_request)?;
    }
    Ok(())
}

fn mod_download_content_base(config: &DstConfig) -> PathBuf {
    PathBuf::from(&config.mod_download_path)
        .join("steamapps")
        .join("workshop")
        .join("content")
        .join("322330")
}

pub(super) fn ugc_content_root(config: &DstConfig, level: &str) -> PathBuf {
    if config.ugc_directory.is_empty() {
        PathBuf::from(&config.force_install_dir)
            .join("ugc_mods")
            .join(&config.cluster)
            .join(level)
            .join("content")
            .join("322330")
    } else {
        PathBuf::from(&config.ugc_directory)
            .join("content")
            .join("322330")
    }
}

pub(super) fn ugc_acf_location(config: &DstConfig, level: &str) -> (PathBuf, PathBuf) {
    if config.ugc_directory.is_empty() {
        (
            PathBuf::from(&config.force_install_dir)
                .join("ugc_mods")
                .join(&config.cluster)
                .join(level),
            PathBuf::from("appworkshop_322330.acf"),
        )
    } else {
        (
            PathBuf::from(&config.ugc_directory),
            PathBuf::from("appworkshop_322330.acf"),
        )
    }
}

pub(super) fn parse_acf_file(
    root: &Path,
    base: &Path,
    file_name: &Path,
) -> AppResult<BTreeMap<String, AcfWorkshopItem>> {
    if !configured_directory_exists(root, base)? {
        return Ok(BTreeMap::new());
    }
    let Some(mut file) =
        safe_open_optional_existing_file_under_base(base, file_name).map_err(fs_bad_request)?
    else {
        return Ok(BTreeMap::new());
    };
    let mut contents = String::new();
    file.by_ref()
        .take(MAX_MODINFO_BYTES as u64 + 1)
        .read_to_string(&mut contents)?;
    if contents.len() > MAX_MODINFO_BYTES {
        return Err(AppError::payload_too_large("appworkshop ACF is too large"));
    }
    let mut items = BTreeMap::new();
    let mut current_id = None::<String>;
    let mut current_item = AcfWorkshopItem::default();
    let mut in_items = false;
    for line in contents.lines() {
        let trimmed = line.trim().replace('"', "");
        if trimmed == "WorkshopItemsInstalled" {
            in_items = true;
            continue;
        }
        if !in_items || trimmed == "{" || trimmed == "}" || trimmed.is_empty() {
            continue;
        }
        if trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
            current_id = Some(trimmed);
            continue;
        }
        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 2 {
            continue;
        }
        match parts[0] {
            "timeupdated" => current_item.time_updated = parts[1].parse().unwrap_or_default(),
            "manifest" => current_item.manifest = parts[1].to_owned(),
            "ugchandle" => current_item.ugc_handle = parts[1].to_owned(),
            _ => {}
        }
        if current_item.time_updated != 0
            && let Some(id) = current_id.take()
        {
            items.insert(id, current_item.clone());
            current_item = AcfWorkshopItem::default();
        }
    }
    Ok(items)
}

pub(super) fn validate_any_mod_id(value: &str) -> AppResult<String> {
    if value.parse::<i64>().is_ok() {
        return validate_mod_id(value)
            .map(|value| value.into_string())
            .map_err(|error| AppError::bad_request(error.to_string()));
    }
    validate_filename(value)
        .map(|value| value.into_string())
        .map_err(|error| AppError::bad_request(error.to_string()))
}

pub(super) fn ensure_absolute_dir(root: &Path, directory: &Path) -> AppResult<()> {
    safe_ensure_configured_dir(root, &directory.display().to_string())?;
    Ok(())
}

pub(super) fn configured_directory_exists(root: &Path, directory: &Path) -> AppResult<bool> {
    if directory.starts_with(root) {
        let relative = directory
            .strip_prefix(root)
            .map_err(|_| AppError::bad_request("directory path escapes root"))?;
        return safe_directory_exists_under_base(root, relative).map_err(fs_bad_request);
    }
    safe_directory_path_exists(directory).map_err(fs_bad_request)
}

pub(super) fn fs_bad_request(error: impl std::fmt::Display) -> AppError {
    AppError::bad_request(error.to_string())
}
#[derive(Debug, Clone, Default)]
pub(super) struct AcfWorkshopItem {
    pub(super) time_updated: i64,
    pub(super) manifest: String,
    pub(super) ugc_handle: String,
}
