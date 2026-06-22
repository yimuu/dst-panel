//! Binary entry point for the Rust dst-admin server.
//!
//! Startup intentionally mirrors the Go process order: initialize logging,
//! load `config.yml`, open the configured SQLite database, apply compatibility
//! migrations, then serve the Axum router with trusted peer-address metadata.

use std::num::ParseIntError;

use dst_admin_rust::{
    domain::auth::SessionStore,
    domain::scheduler::runtime::{SchedulerRuntimeContext, spawn_scheduler_runtime},
    infra::config::AppConfig,
    infra::db::{connect_sqlite, migrate},
    infra::logging,
    web::app::{AppState, build_connect_info_service},
};

/// Starts the `dst-admin-rust` HTTP server.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root_path = std::env::current_dir()?;
    let log_path = root_path.join(logging::DEFAULT_LOG_FILE);
    logging::init(&log_path)?;

    let config_path = root_path.join("config.yml");
    let config = AppConfig::from_file(&config_path)?;
    let bind_addr = bind_addr(&config)?;

    let database_path = config.database_path();
    let database_url = database_path.to_string_lossy().into_owned();
    let pool = connect_sqlite(&database_url).await?;
    migrate(&pool).await?;

    let state = AppState::new(config, pool, SessionStore::new(), root_path);
    let _scheduler_handle = spawn_scheduler_runtime(SchedulerRuntimeContext::new(
        state.root_path.clone(),
        state.db.clone(),
        state.command_runner.clone(),
        state.process_snapshot_provider.clone(),
        state.lifecycle_grace_period,
    ));
    let listener = tokio::net::TcpListener::bind((bind_addr.host.as_str(), bind_addr.port)).await?;
    let local_addr = listener.local_addr()?;

    tracing::info!(
        bind_address = %local_addr,
        "starting dst-admin-rust http server"
    );

    axum::serve(listener, build_connect_info_service(state)).await?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BindAddress {
    host: String,
    port: u16,
}

#[derive(Debug, thiserror::Error)]
enum BindAddressError {
    #[error("invalid HTTP port `{value}` in config.yml: {source}")]
    InvalidPort {
        value: String,
        #[source]
        source: ParseIntError,
    },
}

fn bind_addr(config: &AppConfig) -> Result<BindAddress, BindAddressError> {
    let host = if config.bind_address.trim().is_empty() {
        "0.0.0.0".to_owned()
    } else {
        config.bind_address.trim().to_owned()
    };

    let port_value = if config.port.trim().is_empty() {
        "8082"
    } else {
        config.port.trim()
    };
    let port = port_value
        .parse::<u16>()
        .map_err(|source| BindAddressError::InvalidPort {
            value: port_value.to_owned(),
            source,
        })?;

    Ok(BindAddress { host, port })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dst_admin_rust::infra::config::AutoUpdateModinfoConfig;

    fn config_with_bind(bind_address: &str, port: &str) -> AppConfig {
        AppConfig {
            bind_address: bind_address.to_owned(),
            port: port.to_owned(),
            path: String::new(),
            data_dir: String::new(),
            database: "dst-db".to_owned(),
            steamcmd: String::new(),
            steam_api_key: None,
            flag: String::new(),
            wan_ip: String::new(),
            white_admin_ip: None,
            token: None,
            dst_version_url: String::new(),
            auto_update_modinfo: AutoUpdateModinfoConfig {
                enable: false,
                check_interval: 5,
                update_check_interval: 10,
            },
            dst_cli_port: String::new(),
        }
    }

    #[test]
    fn bind_addr_accepts_ipv6_and_hostname_hosts_like_go_listen() {
        assert!(bind_addr(&config_with_bind("::1", "18082")).is_ok());
        assert!(bind_addr(&config_with_bind("localhost", "18082")).is_ok());
    }

    #[test]
    fn bind_addr_applies_documented_empty_defaults() {
        assert_eq!(
            bind_addr(&config_with_bind("", "")).unwrap(),
            BindAddress {
                host: "0.0.0.0".to_owned(),
                port: 8082,
            }
        );
    }
}
