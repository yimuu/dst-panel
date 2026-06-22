//! `server.ini` parser and renderer compatible with Go's level world shape.

use serde::{Deserialize, Serialize};

use super::{cluster_ini::parse_ini_values, parse_bool, parse_u64};

/// Go `level.ServerIni` JSON shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerIni {
    pub server_port: u64,
    pub is_master: bool,
    pub name: String,
    pub id: u64,
    pub encode_user_path: bool,
    pub authentication_port: u64,
    pub master_server_port: u64,
}

impl ServerIni {
    pub fn master_default() -> Self {
        Self {
            server_port: 10999,
            is_master: true,
            name: "Master".to_owned(),
            id: 10000,
            encode_user_path: true,
            authentication_port: 8766,
            master_server_port: 27016,
        }
    }

    pub fn caves_default() -> Self {
        Self {
            server_port: 10998,
            is_master: false,
            name: "Caves".to_owned(),
            id: 10010,
            encode_user_path: true,
            authentication_port: 8766,
            master_server_port: 27016,
        }
    }

    pub fn from_contents(contents: &str, is_master: bool) -> Self {
        let mut config = if is_master {
            Self::master_default()
        } else {
            Self::caves_default()
        };
        let values = parse_ini_values(contents);
        config.server_port = parse_u64(values.get("server_port"), config.server_port);
        config.is_master = parse_bool(values.get("is_master"), is_master);
        config.name = values.get("name").cloned().unwrap_or_default();
        config.id = parse_u64(values.get("id"), config.id);
        config.encode_user_path = parse_bool(values.get("encode_user_path"), true);
        config.authentication_port = parse_u64(
            values.get("authentication_port"),
            config.authentication_port,
        );
        config.master_server_port =
            parse_u64(values.get("master_server_port"), config.master_server_port);
        config
    }

    pub fn to_ini(&self) -> String {
        format!(
            "[NETWORK]\n\
server_port = {}\n\
\n\
[SHARD]\n\
is_master = {}\n\
name = {}\n\
id = {}\n\
\n\
[ACCOUNT]\n\
encode_user_path = {}\n\
\n\
[STEAM]\n\
master_server_port = {}\n\
authentication_port = {}\n",
            self.server_port,
            self.is_master,
            self.name,
            self.id,
            self.encode_user_path,
            self.master_server_port,
            self.authentication_port,
        )
    }
}
