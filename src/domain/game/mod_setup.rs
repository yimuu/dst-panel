//! Dedicated server mod setup file helpers.
//!
//! Go regenerates `dedicated_server_mods_setup.lua` from level
//! `modoverrides.lua` contents in several workflows. Keeping that filesystem
//! behavior in the game domain lets lifecycle, backup restore, and HTTP config
//! routes share the same safe path and logging behavior.

use std::{
    io::{self, Read},
    path::Path,
};

use crate::{
    dst::{self, DstConfig},
    infra::fs_paths,
    validation::validate_mod_id,
};

pub(crate) fn write_dedicated_server_mods_setup(root: &Path, mod_data: &str) -> io::Result<()> {
    let config = DstConfig::load(root)?;
    let mods_dir = mods_dir_for_config(&config);
    dst::safe_ensure_configured_dir(root, &mods_dir.display().to_string())?;
    let contents = render_dedicated_server_mods_setup(mod_data)?;
    fs_paths::safe_overwrite_file_under_base(
        &mods_dir,
        "dedicated_server_mods_setup.lua",
        contents.as_bytes(),
    )
    .map_err(safe_path_error)?;
    tracing::info!(
        mod_count = workshop_ids(mod_data).len(),
        "updated dedicated_server_mods_setup.lua"
    );
    Ok(())
}

pub(crate) fn merge_dedicated_server_mods_setup(root: &Path, mod_data: &str) -> io::Result<()> {
    if mod_data.is_empty() {
        return Ok(());
    }
    let config = DstConfig::load(root)?;
    let mods_dir = mods_dir_for_config(&config);
    dst::safe_ensure_configured_dir(root, &mods_dir.display().to_string())?;
    let new_lines = server_mod_setup_lines(mod_data)?;
    let mut existing = Vec::new();
    if let Some(mut file) = fs_paths::safe_open_optional_existing_file_under_base(
        &mods_dir,
        "dedicated_server_mods_setup.lua",
    )
    .map_err(safe_path_error)?
    {
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        existing.extend(
            contents
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToOwned::to_owned),
        );
    }

    let mut merged = Vec::new();
    for line in new_lines {
        if !existing.iter().any(|existing_line| existing_line == &line) {
            merged.push(line);
        }
    }
    merged.extend(existing);
    let contents = if merged.is_empty() {
        String::new()
    } else {
        format!("{}\n", merged.join("\n"))
    };
    fs_paths::safe_overwrite_file_under_base(
        &mods_dir,
        "dedicated_server_mods_setup.lua",
        contents.as_bytes(),
    )
    .map_err(safe_path_error)?;
    tracing::info!(
        mod_count = workshop_ids(mod_data).len(),
        "merged dedicated_server_mods_setup.lua"
    );
    Ok(())
}

fn render_dedicated_server_mods_setup(mod_data: &str) -> io::Result<String> {
    let lines = server_mod_setup_lines(mod_data)?;
    if lines.is_empty() {
        return Ok(String::new());
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn server_mod_setup_lines(mod_data: &str) -> io::Result<Vec<String>> {
    let mut rendered = String::new();
    let mut lines = Vec::new();
    for mod_id in workshop_ids(mod_data) {
        let safe_mod_id = validate_mod_id(&mod_id)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        rendered.clear();
        rendered.push_str("ServerModSetup(\"");
        rendered.push_str(safe_mod_id.as_str());
        rendered.push_str("\")");
        lines.push(rendered.clone());
    }
    Ok(lines)
}

fn mods_dir_for_config(config: &DstConfig) -> std::path::PathBuf {
    let mut install_dir = Path::new(&config.force_install_dir).to_path_buf();
    if config.beta == 1 {
        install_dir = Path::new(&format!("{}-beta", install_dir.display())).to_path_buf();
    }
    install_dir.join("mods")
}

fn workshop_ids(mod_data: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = mod_data;
    const PREFIX: &str = "\"workshop-";

    while let Some(start) = rest.find(PREFIX) {
        let after_prefix = &rest[start + PREFIX.len()..];
        let Some(end) = after_prefix.find('"') else {
            break;
        };
        let candidate = &after_prefix[..end];
        let id = candidate.split('-').next().unwrap_or_default().trim();
        if !id.is_empty() {
            ids.push(id.to_owned());
        }
        rest = &after_prefix[end + 1..];
    }

    ids
}

fn safe_path_error(error: fs_paths::FsPathError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, error)
}
