//! Go-compatible online-player queries backed by DST log scraping.
//!
//! The legacy backend asks a running shard to `print` a marker-tagged player
//! table through `screen`, waits briefly, then reads recent `server_log.txt`
//! lines and extracts rows containing that marker and a KU id. This module
//! keeps the same compatibility flow while using existing safe boundaries:
//! process snapshots decide whether a level is running, console commands go
//! through [`super::console::send_level_command`], and log reads use the
//! bounded reverse reader from [`crate::logs`].

use std::{
    collections::HashSet,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

use crate::{
    dst,
    infra::command::CommandRunner,
    infra::process::{ProcessSnapshotProvider, first_level_process},
    logs::recent_lines_from_file,
    validation::validate_level_name,
    web::error::{AppError, AppResult},
};

use super::console;

const ALL_LEVEL_MARKER: &str = "#ALL_LEVEL";
const DEFAULT_QUERY_LOG_LINES: usize = 200;
const MASTER_LEVEL: &str = "Master";

/// Go `PlayerVO` response shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PlayerVo {
    key: String,
    day: String,
    name: String,
    #[serde(rename = "kuId")]
    ku_id: String,
    role: String,
}

/// Queries one shard for online players using Go's marker-and-log flow.
pub(crate) async fn query_online_players(
    root: &Path,
    runner: &dyn CommandRunner,
    process_provider: &dyn ProcessSnapshotProvider,
    level_name: &str,
    delay: Duration,
    marker_override: Option<&str>,
) -> AppResult<Vec<PlayerVo>> {
    let all_levels = level_name == ALL_LEVEL_MARKER;
    let command_level = if all_levels {
        MASTER_LEVEL.to_owned()
    } else {
        validate_level_name(level_name)
            .map_err(|error| AppError::bad_request(error.to_string()))?
            .as_str()
            .to_owned()
    };
    let cluster_name = dst::current_cluster_name(root).map_err(file_error("resolve cluster"))?;
    let cluster_dir = dst::current_cluster_dir(root).map_err(file_error("resolve cluster"))?;

    if !level_running(process_provider, &cluster_name, &command_level) {
        tracing::info!(
            cluster_name,
            level_name = %command_level,
            requested_level = level_name,
            "skipping online-player query because level is stopped"
        );
        return Ok(Vec::new());
    }

    let marker = marker_override
        .map(ToOwned::to_owned)
        .unwrap_or_else(default_marker);
    let command = player_lua_command(&marker, all_levels);
    tracing::info!(
        cluster_name,
        level_name = %command_level,
        all_levels,
        marker_len = marker.len(),
        "sending online-player query command"
    );
    console::send_level_command(runner, &cluster_name, &command_level, &command).await?;
    if !delay.is_zero() {
        tokio::time::sleep(delay).await;
    }

    let log_path = Path::new(command_level.as_str()).join("server_log.txt");
    let mut file = match dst::safe_open_cluster_file(&cluster_dir, &log_path) {
        Ok(Some(file)) => file,
        Ok(None) => {
            tracing::warn!(
                cluster_name,
                level_name = %command_level,
                "online-player log is missing or unsafe"
            );
            return Ok(Vec::new());
        }
        Err(error) => {
            tracing::warn!(
                cluster_name,
                level_name = %command_level,
                error = %error,
                "failed to open online-player log"
            );
            return Ok(Vec::new());
        }
    };
    let lines = match recent_lines_from_file(&mut file, DEFAULT_QUERY_LOG_LINES) {
        Ok(lines) => lines,
        Err(error) => {
            tracing::warn!(
                cluster_name,
                level_name = %command_level,
                error = %error,
                "failed to read recent online-player log lines"
            );
            return Ok(Vec::new());
        }
    };
    let players = parse_player_lines(&lines, &marker);
    tracing::info!(
        cluster_name,
        level_name = %command_level,
        line_count = lines.len(),
        player_count = players.len(),
        "parsed online-player query result"
    );
    Ok(players)
}

fn level_running(
    process_provider: &dyn ProcessSnapshotProvider,
    cluster_name: &str,
    level_name: &str,
) -> bool {
    match process_provider.snapshots() {
        Ok(snapshots) => {
            let running = first_level_process(&snapshots, cluster_name, level_name).is_some();
            tracing::debug!(
                cluster_name,
                level_name,
                running,
                snapshot_count = snapshots.len(),
                "checked online-player level process"
            );
            running
        }
        Err(error) => {
            tracing::warn!(
                cluster_name,
                level_name,
                error = %error,
                "failed to collect process snapshots for online-player query"
            );
            false
        }
    }
}

fn player_lua_command(marker: &str, all_levels: bool) -> String {
    if all_levels {
        format!(
            "for key, player in pairs(TheNet:GetClientTable() or {{}}) do if player and player.userid and player.userid ~= 'Host' then print('player: {{[{marker}] ['..tostring(key)..'] [0] ['..tostring(player.userid)..'] ['..tostring(player.name or '')..'] ['..tostring(player.prefab or '')..']}}') end end"
        )
    } else {
        format!(
            "for key, player in pairs(AllPlayers or {{}}) do if player and player.userid and player.userid ~= 'Host' then print('player: {{[{marker}] ['..tostring(key)..'] ['..tostring(TheWorld and TheWorld.state and TheWorld.state.cycles or 0)..'] ['..tostring(player.userid)..'] ['..tostring(player.name or '')..'] ['..tostring(player.prefab or '')..']}}') end end"
        )
    }
}

fn parse_player_lines(lines: &[String], marker: &str) -> Vec<PlayerVo> {
    let mut seen_ku_ids = HashSet::new();
    let mut players = Vec::new();
    for line in lines {
        if !line.contains(marker) || !line.contains("KU") || line.contains("Host") {
            continue;
        }
        let values = bracket_values(line);
        if values.len() < 6 || values[0] != marker {
            continue;
        }
        let ku_id = values[3].clone();
        if !ku_id.starts_with("KU") || !seen_ku_ids.insert(ku_id.clone()) {
            continue;
        }
        players.push(PlayerVo {
            key: values[1].clone(),
            day: values[2].clone(),
            name: values[4].clone(),
            ku_id,
            role: values[5].clone(),
        });
    }
    players
}

fn bracket_values(line: &str) -> Vec<String> {
    let line = line
        .split_once("player: {")
        .map(|(_, payload)| payload)
        .unwrap_or(line);
    let mut values = Vec::new();
    let mut remainder = line;
    while let Some(start) = remainder.find('[') {
        let after_start = &remainder[start + 1..];
        let Some(end) = after_start.find(']') else {
            break;
        };
        values.push(after_start[..end].to_owned());
        remainder = &after_start[end + 1..];
    }
    values
}

fn default_marker() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn file_error(
    operation: &'static str,
) -> impl FnOnce(std::io::Error) -> AppError + Copy + Send + Sync + 'static {
    move |error| {
        tracing::error!(operation, error = %error, "online-player query file operation failed");
        AppError::internal(operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_dedupes_ku_ids_and_ignores_unrelated_lines() {
        let lines = vec![
            "[00:00:02]: player: {[m] [2] [13] [KU_1] [Alice2] [willow]}".to_owned(),
            "[00:00:01]: player: {[m] [1] [12] [KU_1] [Alice] [wilson]}".to_owned(),
            "[00:00:00]: player: {[x] [1] [12] [KU_2] [Bob] [wendy]}".to_owned(),
            "[00:00:00]: player: {[m] [1] [12] [Host] [Host] [wilson]}".to_owned(),
        ];

        let players = parse_player_lines(&lines, "m");

        assert_eq!(players.len(), 1);
        assert_eq!(players[0].ku_id, "KU_1");
        assert_eq!(players[0].name, "Alice2");
    }
}
