//! DST level file workflow helpers.
//!
//! These functions own the Go-compatible `level.json` and per-shard file
//! behavior. HTTP handlers call into this module for request extraction and
//! response wrapping only.

use std::{io, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    dst::{self, lua_files, server_ini::ServerIni},
    validation::validate_level_name,
    web::error::{AppError, AppResult},
};

use super::mod_setup::write_dedicated_server_mods_setup;

/// Go `level.World` JSON shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct World {
    #[serde(rename = "levelName")]
    pub(crate) level_name: String,
    #[serde(rename = "is_master")]
    pub(crate) is_master: bool,
    pub(crate) uuid: String,
    pub(crate) leveldataoverride: String,
    pub(crate) modoverrides: String,
    pub(crate) server_ini: ServerIni,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LevelIndex {
    #[serde(rename = "levelList", default)]
    level_list: Vec<LevelIndexItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LevelIndexItem {
    name: String,
    file: String,
}

pub(crate) fn list_worlds_from_cluster_dir(cluster_dir: &Path) -> AppResult<Vec<World>> {
    let index = load_or_initialize_index(cluster_dir)?;
    let mut worlds = Vec::with_capacity(index.level_list.len());
    for item in index.level_list {
        worlds.push(read_world(cluster_dir, item)?);
    }
    Ok(worlds)
}

/// Reads level metadata without creating Go's missing default files.
///
/// `/api/cluster/level` preserves Go's historical initialization side effect.
/// `/api/game/8level/status` is a read-only process query, so it uses this
/// helper to inspect existing `level.json` or existing Master/Caves folders
/// without writing a default level skeleton.
pub(crate) fn list_existing_worlds_from_cluster_dir(cluster_dir: &Path) -> AppResult<Vec<World>> {
    let Some(index) = load_index(cluster_dir)? else {
        return list_default_existing_worlds(cluster_dir);
    };
    if index.level_list.is_empty() {
        return list_default_existing_worlds(cluster_dir);
    }

    let mut worlds = Vec::with_capacity(index.level_list.len());
    for item in index.level_list {
        worlds.push(read_world(cluster_dir, item)?);
    }
    Ok(worlds)
}

/// Reads worlds for `/share/cluster` using Go's export quirk.
///
/// Go iterates `level.json`, then calls `GetLevel(cluster, item.File)`.
/// `GetLevel` sets `levelName` to the folder name, leaves `uuid` empty, and
/// marks every exported world as non-master even when the folder is `Master`.
pub(crate) fn list_share_worlds_from_cluster_dir(cluster_dir: &Path) -> AppResult<Vec<World>> {
    let index = load_index(cluster_dir)?.unwrap_or_else(LevelIndex::default);
    let items = if index.level_list.is_empty() {
        default_existing_level_items(cluster_dir)?
    } else {
        index.level_list
    };

    let mut worlds = Vec::with_capacity(items.len());
    for item in items {
        worlds.push(read_share_world(cluster_dir, &item.file)?);
    }
    Ok(worlds)
}

/// Reads only explicit entries from `level.json` without creating or inferring
/// default worlds.
///
/// Cleanup-all is destructive, so a missing or empty level index should be a
/// no-op rather than an implicit Master/Caves cleanup. Status-style callers use
/// [`list_existing_worlds_from_cluster_dir`] when they need non-mutating
/// Master/Caves discovery.
pub(crate) fn list_indexed_worlds_from_cluster_dir(cluster_dir: &Path) -> AppResult<Vec<World>> {
    let Some(index) = load_index(cluster_dir)? else {
        return Ok(Vec::new());
    };

    let mut worlds = Vec::with_capacity(index.level_list.len());
    for item in index.level_list {
        worlds.push(read_world(cluster_dir, item)?);
    }
    Ok(worlds)
}

pub(crate) fn save_worlds_to_cluster_dir(
    root: &Path,
    cluster_dir: &Path,
    worlds: Vec<World>,
) -> AppResult<usize> {
    let mut index = LevelIndex::default();
    let mut combined_modoverrides = String::new();
    for world in worlds {
        validate_world(&world)?;
        write_world(cluster_dir, &world)?;
        combined_modoverrides.push_str(&world.modoverrides);
        combined_modoverrides.push('\n');
        index.level_list.push(LevelIndexItem {
            name: world.level_name,
            file: world.uuid,
        });
    }
    save_index(cluster_dir, &index)?;
    write_dedicated_server_mods_setup(root, &combined_modoverrides)
        .map_err(file_error("write dedicated server mods setup"))?;
    Ok(index.level_list.len())
}

pub(crate) fn create_world_in_cluster_dir(
    cluster_dir: &Path,
    mut world: World,
) -> AppResult<World> {
    if world.uuid.trim().is_empty() {
        world.uuid = dst::generate_uuid_v4().map_err(AppError::from)?;
    }
    validate_world(&world)?;
    write_world(cluster_dir, &world)?;

    let mut index = load_index(cluster_dir)?.unwrap_or_default();
    if let Some(existing) = index
        .level_list
        .iter_mut()
        .find(|item| item.file == world.uuid)
    {
        existing.name = world.level_name.clone();
    } else {
        index.level_list.push(LevelIndexItem {
            name: world.level_name.clone(),
            file: world.uuid.clone(),
        });
    }
    save_index(cluster_dir, &index)?;

    Ok(world)
}

pub(crate) fn delete_world_from_cluster_dir(cluster_dir: &Path, level_name: &str) -> AppResult<()> {
    let safe_level = validate_level_name(level_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?;
    dst::safe_ensure_cluster_dir(cluster_dir).map_err(file_error("validate cluster directory"))?;

    let mut index = load_index(cluster_dir)?.unwrap_or_default();
    let original_index = index.clone();
    index
        .level_list
        .retain(|item| item.file != safe_level.as_str());
    let staged_level = stage_level_for_delete(cluster_dir, safe_level.as_str())?;

    if let Err(error) = save_index(cluster_dir, &index) {
        if let Some(staged_level) = staged_level.as_deref()
            && let Err(restore_error) =
                restore_staged_level(cluster_dir, staged_level, safe_level.as_str())
        {
            tracing::error!(
                level_name = safe_level.as_str(),
                staged_level,
                save_error = %error,
                restore_error = %restore_error,
                "failed to restore staged level directory after index save failure"
            );
            return Err(file_error("restore staged level directory")(restore_error));
        }
        return Err(error);
    }

    if let Some(staged_level) = staged_level.as_deref()
        && let Err(error) = dst::safe_remove_cluster_dir(cluster_dir, staged_level)
    {
        tracing::error!(
            level_name = safe_level.as_str(),
            staged_level,
            error = %error,
            "failed to remove staged level directory; attempting metadata rollback"
        );
        if let Err(restore_error) =
            restore_staged_level(cluster_dir, staged_level, safe_level.as_str())
        {
            tracing::error!(
                level_name = safe_level.as_str(),
                staged_level,
                restore_error = %restore_error,
                "preserving removed-index state because staged level restore failed"
            );
            return Err(file_error("delete level directory")(error));
        }
        if let Err(index_error) = save_index(cluster_dir, &original_index) {
            tracing::error!(
                level_name = safe_level.as_str(),
                error = %index_error,
                "failed to restore level index after directory rollback; attempting to preserve removed-index state"
            );
            if let Err(restage_error) =
                dst::safe_rename_cluster_dir(cluster_dir, safe_level.as_str(), staged_level)
            {
                tracing::error!(
                    level_name = safe_level.as_str(),
                    staged_level,
                    error = %restage_error,
                    "failed to restage level directory after index rollback failure"
                );
            }
            return Err(index_error);
        }
        return Err(file_error("delete level directory")(error));
    }

    Ok(())
}

fn list_default_existing_worlds(cluster_dir: &Path) -> AppResult<Vec<World>> {
    let items = default_existing_level_items(cluster_dir)?;
    let mut worlds = Vec::with_capacity(items.len());
    for item in items {
        worlds.push(read_world(cluster_dir, item)?);
    }
    Ok(worlds)
}

fn default_existing_level_items(cluster_dir: &Path) -> AppResult<Vec<LevelIndexItem>> {
    let mut worlds = Vec::new();
    if dst::safe_optional_cluster_dir_exists(cluster_dir, "Master")
        .map_err(file_error("check Master directory"))?
    {
        worlds.push(LevelIndexItem {
            name: "森林".to_owned(),
            file: "Master".to_owned(),
        });
    }
    if dst::safe_optional_cluster_dir_exists(cluster_dir, "Caves")
        .map_err(file_error("check Caves directory"))?
    {
        worlds.push(LevelIndexItem {
            name: "洞穴".to_owned(),
            file: "Caves".to_owned(),
        });
    }
    Ok(worlds)
}

fn stage_level_for_delete(cluster_dir: &Path, level_name: &str) -> AppResult<Option<String>> {
    if !dst::safe_cluster_dir_exists(cluster_dir, level_name)
        .map_err(file_error("validate level directory"))?
    {
        return Ok(None);
    }
    let tombstone = format!(
        "__dst_admin_delete_{}",
        dst::generate_uuid_v4()
            .map_err(AppError::from)?
            .replace('-', "")
    );
    dst::safe_rename_cluster_dir(cluster_dir, level_name, &tombstone)
        .map_err(file_error("stage level directory for delete"))?;
    Ok(Some(tombstone))
}

fn restore_staged_level(
    cluster_dir: &Path,
    staged_level: &str,
    level_name: &str,
) -> io::Result<()> {
    dst::safe_rename_cluster_dir(cluster_dir, staged_level, level_name)?;
    tracing::warn!(
        staged_level,
        level_name,
        "restored staged level directory after delete rollback"
    );
    Ok(())
}

fn validate_world(world: &World) -> AppResult<()> {
    validate_level_name(&world.uuid).map_err(|error| AppError::bad_request(error.to_string()))?;
    if world.level_name.trim().is_empty() {
        return Err(AppError::bad_request("levelName is required"));
    }
    Ok(())
}

fn write_world(cluster_dir: &Path, world: &World) -> AppResult<()> {
    dst::write_world_files(
        cluster_dir,
        &world.uuid,
        &world.leveldataoverride,
        &world.modoverrides,
        &world.server_ini,
    )
    .map_err(file_error("write level files"))
}

fn read_world(cluster_dir: &Path, item: LevelIndexItem) -> AppResult<World> {
    let level_name = validate_level_name(&item.file)
        .map_err(|error| AppError::bad_request(error.to_string()))?;
    let level_dir = Path::new(level_name.as_str());
    let is_master = item.file.eq_ignore_ascii_case("Master");
    let server_ini_contents =
        dst::safe_read_cluster_file_to_string(cluster_dir, level_dir.join("server.ini"))
            .map_err(file_error("read server.ini"))?;
    Ok(World {
        level_name: item.name,
        is_master,
        uuid: item.file,
        leveldataoverride: lua_files::contents_or_default(
            dst::safe_read_cluster_file_to_string(
                cluster_dir,
                level_dir.join("leveldataoverride.lua"),
            )
            .map_err(file_error("read leveldataoverride"))?,
        ),
        modoverrides: lua_files::contents_or_default(
            dst::safe_read_cluster_file_to_string(cluster_dir, level_dir.join("modoverrides.lua"))
                .map_err(file_error("read modoverrides"))?,
        ),
        server_ini: server_ini_contents
            .as_deref()
            .map(|contents| ServerIni::from_contents(contents, is_master))
            .unwrap_or_else(|| {
                if is_master {
                    ServerIni::master_default()
                } else {
                    ServerIni::caves_default()
                }
            }),
    })
}

fn read_share_world(cluster_dir: &Path, level_name: &str) -> AppResult<World> {
    let level_name = validate_level_name(level_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?;
    let level_dir = Path::new(level_name.as_str());
    let server_ini_contents =
        dst::safe_read_cluster_file_to_string(cluster_dir, level_dir.join("server.ini"))
            .map_err(file_error("read share server.ini"))?;
    Ok(World {
        level_name: level_name.as_str().to_owned(),
        is_master: false,
        uuid: String::new(),
        leveldataoverride: lua_files::contents_or_default(
            dst::safe_read_cluster_file_to_string(
                cluster_dir,
                level_dir.join("leveldataoverride.lua"),
            )
            .map_err(file_error("read share leveldataoverride"))?,
        ),
        modoverrides: lua_files::contents_or_default(
            dst::safe_read_cluster_file_to_string(cluster_dir, level_dir.join("modoverrides.lua"))
                .map_err(file_error("read share modoverrides"))?,
        ),
        server_ini: server_ini_contents
            .as_deref()
            .map(|contents| ServerIni::from_contents(contents, false))
            .unwrap_or_else(ServerIni::caves_default),
    })
}

fn load_or_initialize_index(cluster_dir: &Path) -> AppResult<LevelIndex> {
    if let Some(index) = load_index(cluster_dir)?
        && !index.level_list.is_empty()
    {
        return Ok(index);
    }

    dst::safe_ensure_cluster_dir(cluster_dir).map_err(file_error("validate cluster directory"))?;
    let mut index = LevelIndex::default();
    if dst::safe_cluster_dir_exists(cluster_dir, "Master")
        .map_err(file_error("check Master directory"))?
    {
        index.level_list.push(LevelIndexItem {
            name: "森林".to_owned(),
            file: "Master".to_owned(),
        });
    } else {
        let master = World {
            level_name: "森林".to_owned(),
            is_master: true,
            uuid: "Master".to_owned(),
            leveldataoverride: "return {}".to_owned(),
            modoverrides: "return {}".to_owned(),
            server_ini: ServerIni::master_default(),
        };
        write_world(cluster_dir, &master)?;
        index.level_list.push(LevelIndexItem {
            name: master.level_name,
            file: master.uuid,
        });
    }
    if dst::safe_cluster_dir_exists(cluster_dir, "Caves")
        .map_err(file_error("check Caves directory"))?
    {
        index.level_list.push(LevelIndexItem {
            name: "洞穴".to_owned(),
            file: "Caves".to_owned(),
        });
    }
    save_index(cluster_dir, &index)?;
    Ok(index)
}

fn load_index(cluster_dir: &Path) -> AppResult<Option<LevelIndex>> {
    let Some(contents) = dst::safe_read_cluster_file_to_string(cluster_dir, "level.json")
        .map_err(file_error("read level.json"))?
    else {
        return Ok(None);
    };
    let index = serde_json::from_str(&contents).map_err(|error| {
        tracing::error!(error = %error, "failed to parse DST level.json");
        AppError::internal("parse level.json")
    })?;
    Ok(Some(index))
}

fn save_index(cluster_dir: &Path, index: &LevelIndex) -> AppResult<()> {
    let contents =
        serde_json::to_string(index).map_err(|_| AppError::internal("encode level.json"))?;
    dst::safe_write_cluster_file(cluster_dir, "level.json", contents)
        .map_err(file_error("write level.json"))
}

fn file_error(
    operation: &'static str,
) -> impl FnOnce(io::Error) -> AppError + Copy + Send + Sync + 'static {
    move |error| {
        if error.kind() == io::ErrorKind::InvalidInput {
            AppError::bad_request(error.to_string())
        } else {
            tracing::error!(operation, error = %error, "level file operation failed");
            AppError::internal(operation)
        }
    }
}
