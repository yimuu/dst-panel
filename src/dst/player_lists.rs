//! Line-oriented admin, white, and block list file helpers.

use std::{io, path::Path};

use serde::Deserialize;

use crate::validation::validate_ku_id;

use super::{safe_read_cluster_file_to_string, safe_write_cluster_file};

#[derive(Debug, Deserialize)]
pub struct AdminListRequest {
    #[serde(rename = "adminList")]
    pub admin_list: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct BlacklistRequest {
    pub blacklist: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct WhitelistRequest {
    pub whitelist: Vec<String>,
}

pub fn read_in_cluster(cluster_dir: &Path, file_name: &str) -> io::Result<Vec<String>> {
    let Some(contents) = safe_read_cluster_file_to_string(cluster_dir, file_name)? else {
        tracing::info!(file_name, "player list file missing; returning empty list");
        return Ok(Vec::new());
    };
    Ok(parse_lines(&contents))
}

pub fn overwrite_in_cluster(
    cluster_dir: &Path,
    file_name: &str,
    values: &[String],
) -> io::Result<()> {
    let values = validated_values(values)?;
    safe_write_cluster_file(cluster_dir, file_name, render_lines(&values))
}

pub fn append_unique_in_cluster(
    cluster_dir: &Path,
    file_name: &str,
    values: &[String],
) -> io::Result<()> {
    let mut current = read_in_cluster(cluster_dir, file_name)?;
    for value in validated_values(values)? {
        if !current.iter().any(|existing| existing == &value) {
            current.push(value);
        }
    }
    safe_write_cluster_file(cluster_dir, file_name, render_lines(&current))
}

pub fn remove_values_in_cluster(
    cluster_dir: &Path,
    file_name: &str,
    values: &[String],
) -> io::Result<()> {
    let remove = validated_values(values)?;
    let current = read_in_cluster(cluster_dir, file_name)?;
    let retained: Vec<_> = current
        .into_iter()
        .filter(|value| !remove.iter().any(|candidate| candidate == value))
        .collect();
    safe_write_cluster_file(cluster_dir, file_name, render_lines(&retained))
}

fn validated_values(values: &[String]) -> io::Result<Vec<String>> {
    values
        .iter()
        .map(|value| {
            validate_ku_id(value)
                .map(|safe| safe.as_str().to_owned())
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
        })
        .collect()
}

fn parse_lines(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn render_lines(lines: &[String]) -> String {
    let mut contents = lines.join("\n");
    if !contents.is_empty() {
        contents.push('\n');
    }
    contents
}
