//! `cluster.ini` parser and renderer compatible with the Go templates.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{parse_bool, parse_u64};

/// Go `level.ClusterIni` JSON shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterIni {
    pub game_mode: String,
    pub max_players: u64,
    pub pvp: bool,
    pub pause_when_nobody: bool,
    pub vote_enabled: bool,
    pub vote_kick_enabled: bool,
    pub lan_only_cluster: bool,
    pub cluster_intention: String,
    pub cluster_description: String,
    pub cluster_password: String,
    pub cluster_name: String,
    pub offline_cluster: bool,
    pub cluster_language: String,
    pub whitelist_slots: u64,
    pub tick_rate: u64,
    pub console_enabled: bool,
    pub max_snapshots: u64,
    pub shard_enabled: bool,
    pub bind_ip: String,
    pub master_ip: String,
    pub master_port: u64,
    pub cluster_key: String,
    pub steam_group_id: String,
    pub steam_group_only: bool,
    pub steam_group_admins: bool,
}

impl ClusterIni {
    /// Returns Go's defaults from `level.NewClusterIni`.
    pub fn default_for_new_cluster() -> Self {
        Self {
            game_mode: "survival".to_owned(),
            max_players: 8,
            pvp: false,
            pause_when_nobody: true,
            vote_enabled: true,
            vote_kick_enabled: true,
            lan_only_cluster: false,
            cluster_intention: String::new(),
            cluster_description: String::new(),
            cluster_password: String::new(),
            cluster_name: "我的饥荒服务世界".to_owned(),
            offline_cluster: false,
            cluster_language: "zh".to_owned(),
            whitelist_slots: 0,
            tick_rate: 15,
            console_enabled: true,
            max_snapshots: 6,
            shard_enabled: true,
            bind_ip: "0.0.0.0".to_owned(),
            master_ip: "127.0.0.1".to_owned(),
            master_port: 10888,
            cluster_key: String::new(),
            steam_group_id: String::new(),
            steam_group_only: false,
            steam_group_admins: false,
        }
    }

    /// Parses `cluster.ini` contents, applying Go defaults for missing keys.
    pub fn from_contents(contents: &str) -> Self {
        let values = parse_ini_values(contents);
        let mut config = Self::default_for_new_cluster();
        config.game_mode = values
            .get("game_mode")
            .filter(|value| !value.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| "survival".to_owned());
        config.max_players = parse_u64(values.get("max_players"), 8);
        config.pvp = parse_bool(values.get("pvp"), false);
        config.pause_when_nobody = parse_bool(values.get("pause_when_empty"), true);
        config.vote_enabled = parse_bool(values.get("vote_enabled"), true);
        config.vote_kick_enabled = parse_bool(values.get("vote_kick_enabled"), true);
        config.lan_only_cluster = parse_bool(values.get("lan_only_cluster"), false);
        config.cluster_intention = value(&values, "cluster_intention");
        config.cluster_password = value(&values, "cluster_password");
        config.cluster_description = value(&values, "cluster_description");
        config.cluster_name = value(&values, "cluster_name");
        config.offline_cluster = parse_bool(values.get("offline_cluster"), false);
        config.cluster_language = values
            .get("cluster_language")
            .cloned()
            .unwrap_or_else(|| "zh".to_owned());
        config.whitelist_slots = parse_u64(values.get("whitelist_slots"), 0);
        config.tick_rate = parse_u64(values.get("tick_rate"), 15);
        config.console_enabled = parse_bool(values.get("console_enabled"), true);
        config.max_snapshots = parse_u64(values.get("max_snapshots"), 6);
        config.shard_enabled = parse_bool(values.get("shard_enabled"), true);
        config.bind_ip = values
            .get("bind_ip")
            .cloned()
            .unwrap_or_else(|| "127.0.0.1".to_owned());
        config.master_ip = values
            .get("master_ip")
            .cloned()
            .unwrap_or_else(|| "127.0.0.1".to_owned());
        config.master_port = parse_u64(values.get("master_port"), 10888);
        config.cluster_key = value(&values, "cluster_key");
        config.steam_group_id = value(&values, "steam_group_id");
        config.steam_group_only = parse_bool(values.get("steam_group_only"), false);
        config.steam_group_admins = parse_bool(values.get("steam_group_admins"), false);
        config
    }

    /// Renders a Go-template-compatible `cluster.ini`.
    pub fn to_ini(&self) -> String {
        format!(
            "[GAMEPLAY]\n\
game_mode = {}\n\
max_players = {}\n\
pvp = {}\n\
pause_when_empty = {}\n\
vote_enabled = {}\n\
vote_kick_enabled = {}\n\
\n\
[NETWORK]\n\
lan_only_cluster = {}\n\
cluster_intention = {}\n\
cluster_password = {}\n\
cluster_description = {}\n\
cluster_name = {}\n\
offline_cluster = {}\n\
cluster_language = {}\n\
whitelist_slots = {}\n\
tick_rate = {}\n\
\n\
[MISC]\n\
console_enabled = {}\n\
max_snapshots = {}\n\
\n\
[SHARD]\n\
shard_enabled = {}\n\
bind_ip = {}\n\
master_ip = {}\n\
master_port = {}\n\
cluster_key = {}\n\
\n\
[STEAM]\n\
steam_group_only = {}\n\
steam_group_id = {}\n\
steam_group_admins = {}\n",
            self.game_mode,
            self.max_players,
            self.pvp,
            self.pause_when_nobody,
            self.vote_enabled,
            self.vote_kick_enabled,
            self.lan_only_cluster,
            self.cluster_intention,
            self.cluster_password,
            self.cluster_description,
            self.cluster_name,
            self.offline_cluster,
            self.cluster_language,
            self.whitelist_slots,
            self.tick_rate,
            self.console_enabled,
            self.max_snapshots,
            self.shard_enabled,
            self.bind_ip,
            self.master_ip,
            self.master_port,
            self.cluster_key,
            self.steam_group_only,
            self.steam_group_id,
            self.steam_group_admins,
        )
    }
}

pub(crate) fn parse_ini_values(contents: &str) -> HashMap<String, String> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_owned(), value.trim().to_owned()))
        })
        .collect()
}

fn value(values: &HashMap<String, String>, key: &str) -> String {
    values.get(key).cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::ClusterIni;

    #[test]
    fn cluster_ini_defaults_empty_game_mode_to_survival() {
        assert_eq!(ClusterIni::default_for_new_cluster().game_mode, "survival");
        assert_eq!(ClusterIni::from_contents("").game_mode, "survival");
        assert_eq!(
            ClusterIni::from_contents("[GAMEPLAY]\ngame_mode = \n").game_mode,
            "survival"
        );
    }
}
