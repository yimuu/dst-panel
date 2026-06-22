use std::fs;

use axum::{body::to_bytes, response::IntoResponse};
use dst_admin_rust::{infra::config::AppConfig, web::error::AppError};
use tempfile::tempdir;

#[test]
fn loads_existing_config_keys_and_applies_go_defaults() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yml");
    fs::write(
        &config_path,
        r#"
bindAddress: "127.0.0.1"
port: 18082
database: dst-db
whiteadminip: "127.0.0.1"
"#,
    )
    .unwrap();

    let config = AppConfig::from_file(&config_path).unwrap();

    assert_eq!(config.bind_address, "127.0.0.1");
    assert_eq!(config.port, "18082");
    assert_eq!(config.database, "dst-db");
    assert_eq!(config.white_admin_ip.as_deref(), Some("127.0.0.1"));
    assert_eq!(config.auto_update_modinfo.check_interval, 5);
    assert_eq!(config.auto_update_modinfo.update_check_interval, 10);
    assert_eq!(
        config.dst_version_url,
        "https://api.dstserverlist.top/api/v2/Server/Version"
    );
}

#[test]
fn loads_main_data_dir_config_key() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yml");
    fs::write(
        &config_path,
        r#"
bindAddress: "127.0.0.1"
port: 18082
dataDir: "/srv/dst-admin"
database: dst-db
"#,
    )
    .unwrap();

    let config = AppConfig::from_file(&config_path).unwrap();

    assert_eq!(config.data_dir, "/srv/dst-admin");
}

#[test]
fn database_path_uses_main_data_dir_like_go() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yml");
    fs::write(
        &config_path,
        r#"
bindAddress: "127.0.0.1"
port: 18082
dataDir: "/srv/dst-admin"
database: dst-db
"#,
    )
    .unwrap();

    let config = AppConfig::from_file(&config_path).unwrap();

    assert_eq!(
        config.database_path(),
        std::path::PathBuf::from("/srv/dst-admin/dst-db")
    );
}

#[test]
fn missing_config_file_returns_contextual_error() {
    let dir = tempdir().unwrap();
    let err = AppConfig::from_file(dir.path().join("missing.yml")).unwrap_err();
    assert!(err.to_string().contains("missing.yml"));
}

#[test]
fn app_config_debug_redacts_secret_values() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yml");
    fs::write(
        &config_path,
        r#"
bindAddress: "127.0.0.1"
port: 18082
database: dst-db
steamAPIKey: "steam-secret-key"
token: "cluster-secret-token"
"#,
    )
    .unwrap();

    let config = AppConfig::from_file(&config_path).unwrap();
    let debug = format!("{config:?}");

    assert!(debug.contains("steam_api_key"));
    assert!(debug.contains("token"));
    assert!(!debug.contains("steam-secret-key"));
    assert!(!debug.contains("cluster-secret-token"));
}

#[tokio::test]
async fn malformed_secret_config_errors_do_not_leak_secret_values() {
    assert_malformed_secret_config_is_redacted("steamAPIKey", "steam-secret-key").await;
    assert_malformed_secret_config_is_redacted("token", "cluster-secret-token").await;
}

async fn assert_malformed_secret_config_is_redacted(field: &str, secret: &str) {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yml");
    fs::write(
        &config_path,
        format!(
            r#"
bindAddress: "127.0.0.1"
port: 18082
database: dst-db
{field}: ["{secret}"]
"#
        ),
    )
    .unwrap();

    let err = AppConfig::from_file(&config_path).unwrap_err();
    let display = err.to_string();
    let debug = format!("{err:?}");

    assert!(!display.contains(secret));
    assert!(!debug.contains(secret));

    let response = AppError::from(err).into_response();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(!response_body.contains(secret));

    let logged = capture_tracing_output(|| {
        let err = AppConfig::from_file(&config_path).unwrap_err();
        let _response = AppError::from(err).into_response();
    });
    assert!(!logged.contains(secret));
}

fn capture_tracing_output(action: impl FnOnce()) -> String {
    let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let writer = CapturedWriter {
        captured: std::sync::Arc::clone(&captured),
    };
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();

    tracing::subscriber::with_default(subscriber, action);

    String::from_utf8(captured.lock().unwrap().clone()).unwrap()
}

#[derive(Clone)]
struct CapturedWriter {
    captured: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
}

impl std::io::Write for CapturedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.captured.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
