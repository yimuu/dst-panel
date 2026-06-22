# Rust Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust 2024 backend named `dst-admin-rust` inside the current repository while preserving the existing Go backend's API, configuration, SQLite data, file formats, and deployment shape.

**Architecture:** Add a root Rust project with an axum HTTP server, sqlx SQLite layer, explicit compatibility routes, and business modules that replace Go services incrementally. Keep Go code as a reference until Rust parity is verified, then update build/Docker entrypoints and remove Go.

**Tech Stack:** Rust 2024, axum, tokio, sqlx/sqlite, serde/serde_yaml, tower-http, tracing, reqwest, uuid, chrono, tempfile for tests.

---

## Execution Scope

The migration is larger than one safe code change. Execute it as a sequence of independently verified slices:

1. Foundation and low-risk API parity in this plan.
2. Logs/SSE/WebSocket.
3. DST process control.
4. Mods, third-party APIs, map generation, and backup restore.
5. Scheduling and auto-check.
6. Release update and Go removal.

This document contains the full roadmap plus the detailed executable plan for slice 1. Later slices must get their own task-level plans before implementation.

## Cross-Cutting Requirements

- Every public Rust module starts with a `//!` module comment.
- Public Rust structs, enums, traits, and functions have `///` doc comments.
- Compatibility logic has short inline comments explaining the Go behavior being preserved.
- Use `tracing` logs for startup, config, database migration, auth, validation failures, external command intent, and background task lifecycle.
- Never log passwords, session cookies, cluster tokens, Steam API keys, or raw user console commands.
- Use tests first for behavior changes. Run the failing test before writing the implementation.
- Commit after each task that produces a coherent working state.

## Compatibility Details From Current Go Backend

### Route Manifest

Slice 1 must create a compatibility manifest that registers every current Go route explicitly. Low-risk routes are implemented in slice 1; the rest return HTTP 501 with a Go-compatible envelope until their slice migrates the behavior.

Implemented in slice 1:

- `GET /hello`
- `GET /`
- `HEAD /`
- `GET /assets/*filepath`
- `HEAD /assets/*filepath`
- `GET /misc/*filepath`
- `HEAD /misc/*filepath`
- `GET /static/js/*filepath`
- `HEAD /static/js/*filepath`
- `GET /static/css/*filepath`
- `HEAD /static/css/*filepath`
- `GET /static/img/*filepath`
- `HEAD /static/img/*filepath`
- `GET /static/fonts/*filepath`
- `HEAD /static/fonts/*filepath`
- `GET /static/media/*filepath`
- `HEAD /static/media/*filepath`
- `GET /favicon.ico`
- `HEAD /favicon.ico`
- `GET /asset-manifest.json`
- `HEAD /asset-manifest.json`
- `POST /api/login`
- `GET /api/logout`
- `POST /api/change/password`
- `GET /api/user`
- `POST /api/user`
- `GET /api/init`
- `GET /api/kv`
- `POST /api/kv`
- `GET /api/web/link`
- `POST /api/web/link`
- `DELETE /api/web/link`

Registered as explicit compatibility stubs in slice 1:

- `POST /api/init`
- `GET /api/install/steamcmd`
- `GET /api/dst/config`
- `POST /api/dst/config`
- `GET /api/game/config`
- `POST /api/game/config`
- `POST /webhook`
- `GET /api/game/update`
- `GET /api/game/system/info`
- `GET /api/game/system/info/stream`
- `GET /api/game/sent/broadcast`
- `GET /api/game/kick/player`
- `GET /api/game/kill/player`
- `GET /api/game/respawn/player`
- `GET /api/game/rollback`
- `GET /api/game/regenerateworld`
- `POST /api/game/master/console`
- `POST /api/game/caves/console`
- `GET /api/game/operate/player`
- `GET /api/game/backup/restore`
- `GET /api/game/archive`
- `GET /api/game/clean`
- `GET /api/game/clean/level`
- `GET /api/game/clean/level/all`
- `GET /api/game/announce/setting`
- `POST /api/game/announce/setting`
- `GET /api/game/level/server/log`
- `GET /api/game/level/server/chat/log`
- `GET /api/game/level/server/download`
- `GET /api/game/dst-admin-go/log`
- `GET /api/game/dst-admin-go/log/download`
- `GET /api/game/backup`
- `POST /api/game/backup`
- `DELETE /api/game/backup`
- `PUT /api/game/backup`
- `GET /api/game/backup/download`
- `POST /api/game/backup/upload`
- `POST /api/game/backup/snapshot/setting`
- `GET /api/game/backup/snapshot/setting`
- `GET /api/game/backup/snapshot/list`
- `GET /api/cluster`
- `POST /api/cluster`
- `PUT /api/cluster`
- `DELETE /api/cluster`
- `GET /api/cluster/level`
- `PUT /api/cluster/level`
- `POST /api/cluster/level`
- `DELETE /api/cluster/level`
- `GET /api/game/8level/status`
- `GET /api/game/8level/status/stream`
- `GET /api/game/8level/start`
- `GET /api/game/8level/stop`
- `GET /api/game/8level/start/all`
- `GET /api/game/8level/stop/all`
- `GET /api/game/8level/clusterIni`
- `POST /api/game/8level/clusterIni`
- `GET /api/game/8level/players`
- `GET /api/game/8level/players/all`
- `GET /api/game/8level/adminilist`
- `POST /api/game/8level/adminilist`
- `GET /api/game/8level/whitelist`
- `POST /api/game/8level/whitelist`
- `GET /api/game/8level/blacklist`
- `POST /api/game/8level/blacklist`
- `POST /api/game/8level/command`
- `GET /api/game/8level/udp/port`
- `GET /api/game/preinstall`
- `GET /api/share/keyCer`
- `GET /api/share/keyCer/reflush`
- `GET /api/share/keyCer/enable`
- `POST /api/share/cluster/import`
- `GET /share/cluster`
- `GET /api/mod/search`
- `GET /api/mod`
- `GET /api/mod/:modId`
- `PUT /api/mod/:modId`
- `DELETE /api/mod/:modId`
- `DELETE /api/mod/setup/workshop`
- `GET /api/mod/modinfo/:modId`
- `POST /api/mod/modinfo`
- `PUT /api/mod/modinfo`
- `POST /api/mod/modinfo/file`
- `GET /api/mod/ugc/acf`
- `DELETE /api/mod/ugc`
- `POST /api/file/ugc/upload`
- `GET /api/file/background`
- `POST /api/file/background`
- `GET /api/game/player`
- `GET /api/game/player/adminlist`
- `POST /api/game/player/adminlist`
- `DELETE /api/game/player/adminlist`
- `GET /api/game/player/blacklist`
- `POST /api/game/player/blacklist`
- `DELETE /api/game/player/blacklist`
- `GET /api/player/log`
- `POST /api/player/log/delete`
- `GET /api/statistics/active/user`
- `GET /api/statistics/top/death`
- `GET /api/statistics/top/login`
- `GET /api/statistics/top/active`
- `GET /api/statistics/rate/role`
- `GET /api/statistics/regenerate`
- `GET /api/dst/version`
- `POST /api/dst/home/server`
- `POST /api/dst/home/server/detail`
- `GET /api/dst/lobby/server/detail`
- `GET /api/dst/home/server2`
- `GET /api/dst/home/server/detail2`
- `ANY /api/dst-static/*filepath`
- `GET /steam/dst/news`
- `GET /api/task`
- `POST /api/task`
- `DELETE /api/task`
- `GET /api/task/instruct`
- `GET /api/auto/check2`
- `POST /api/auto/check2`
- `GET /api/dst/map/gen`
- `GET /api/dst/map/image`
- `GET /api/dst/map/has/walrusHut/plains`
- `GET /api/dst/map/session/file`
- `GET /api/dst/map/player/session/file`
- `GET /api/game/log/stream`
- `GET /ws`

Streaming, WebSocket, file, upload, and raw proxy routes remain explicit stubs in slice 1, not JSON-compatible fake implementations:

- SSE: `/api/install/steamcmd`, `/api/game/system/info/stream`, `/api/game/8level/status/stream`, `/api/game/log/stream`
- WebSocket: `/ws`
- Downloads: `/api/game/backup/download`, `/api/game/level/server/download`, `/api/game/dst-admin-go/log/download`
- File responses: `/api/file/background`, `/api/dst/map/image`
- Uploads: `/api/game/backup/upload`, `/api/file/ugc/upload`, `/api/file/background`
- Raw proxy: `/api/dst/version`, `/api/dst/home/server`, `/api/dst/home/server/detail`, `/api/dst/home/server2`, `/api/dst/home/server/detail2`, `/api/dst-static/*filepath`

### Database and File Format Details

Main SQLite tables use GORM default plural snake_case names. Every active model embeds `gorm.Model`, so Rust migrations must create and preserve:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `created_at`
- `updated_at`
- `deleted_at`

Repositories must filter `deleted_at IS NULL` and soft-delete by setting `deleted_at` instead of hard-deleting rows.

Active migrated tables and fields:

- `spawns`: `name`, `role`, `time`, `cluster_name`
- `player_logs`: `name`, `role`, `ku_id`, `steam_id`, `time`, `action`, `action_desc`, `ip`, `cluster_name`
- `connects`: `ip`, `name`, `ku_id`, `steam_id`, `time`, `cluster_name`, `session_file`
- `regenerates`: `cluster_name`
- `mod_infos`: `auth`, `consumer_appid`, `creator_appid`, `description`, `file_url`, `modid`, `img`, `last_time`, `mod_config`, `name`, `v`, `update`
- `clusters`: `cluster_name`, `description`, `steam_cmd`, `force_install_dir`, `backup`, `mod_download_path`, `uuid`, `beta`, `bin`, `ugc_directory`, `persistent_storage_root`, `conf_dir`
- `job_tasks`: `cluster_name`, `level_name`, `uuid`, `cron`, `category`, `comment`, `announcement`, `sleep`, `times`, `script`
- `auto_checks`: `name`, `cluster_name`, `level_name`, `uuid`, `enable`, `announcement`, `times`, `sleep`, `interval`, `check_type`
- `announces`: `enable`, `frequency`, `interval`, `interval_unit`, `method`, `content`
- `web_links`: `title`, `url`, `width`, `height`
- `backup_snapshots`: `name`, `interval`, `max_snapshots`, `enable`, `is_c_save`
- `log_records`: `action`, `cluster_name`, `level_name`
- `kvs`: `key`, `value`

Defined in Go but not migrated by current bootstrap:

- `backups`: `name`, `description`, `path`, `size`, `days`, `season`
- `mod_kvs`: `user_id`, `mod_id`, `config`, `version`

Important compatibility notes:

- `clusters.cluster_name` is unique.
- `mod_infos.mod_config` can be stored as text/json affinity on SQLite.
- `mod_infos.update` must be quoted in SQL where required because `update` is a keyword.
- `model.Action` values are integers.
- SQLite booleans should be treated as integer/bool affinity.
- `password.txt` is line-order compatible and must tolerate both `username=admin` and `username = admin`.
- `dst_config` is line-based `key=value`; preserve defaults for `cluster=Cluster1`, `bin=32`, `beta=0`, backup path, and mod download path.
- DST text files include `cluster.ini`, `cluster_token.txt`, `level.json`, list files, `leveldataoverride.lua`, `modoverrides.lua`, per-level `server.ini`, server logs, backup zips, `snapshotMd5`, `dist/assets/background.png`, and `./key`.
- Backup restore code in later slices must validate zip paths against zip-slip.

## File Structure

Create:

- `Cargo.toml` - Rust package, binary name `dst-admin-rust`, dependency declarations.
- `src/main.rs` - process entrypoint.
- `src/lib.rs` - library module tree used by tests and binary.
- `src/app.rs` - application state and router assembly.
- `src/config.rs` - `config.yml` parsing and defaults.
- `src/logging.rs` - tracing setup for console and `dst-admin-go.log`.
- `src/error.rs` - application error type and HTTP mapping.
- `src/response.rs` - Go-compatible JSON response envelopes.
- `src/db.rs` - SQLite pool creation and schema migration.
- `src/models.rs` - database row and DTO types.
- `src/auth.rs` - password file parsing/writing, session store, auth helpers.
- `src/validation.rs` - safe identifier/path validation.
- `src/repositories/mod.rs` - repository module exports.
- `src/repositories/kv.rs` - KV persistence.
- `src/repositories/web_link.rs` - web link persistence.
- `src/repositories/cluster.rs` - cluster persistence.
- `src/handlers/mod.rs` - handler module exports.
- `src/handlers/auth.rs` - login/logout/user/password handlers.
- `src/handlers/init.rs` - init state handlers.
- `src/handlers/kv.rs` - KV handlers.
- `src/handlers/web_link.rs` - web link handlers.
- `src/handlers/compat.rs` - compatibility route registration and explicit not-yet-migrated responses.
- `src/handlers/static_files.rs` - static asset routing behavior.
- `tests/config_tests.rs` - config compatibility tests.
- `tests/auth_tests.rs` - password/session behavior tests.
- `tests/db_tests.rs` - migration/repository tests.
- `tests/http_tests.rs` - route and auth integration tests.
- `tests/compat_manifest_tests.rs` - route manifest smoke tests.

Modify:

- `.gitignore` - ignore Rust `target/` and local SQLite/log artifacts if missing.
- `build_linux.sh` - build `dst-admin-rust` for Linux.
- `build_window.sh` - build `dst-admin-rust` for Windows.
- `Dockerfile` - copy `dst-admin-rust` instead of `dst-admin-go`.
- `docker-entrypoint.sh` - execute `./dst-admin-rust`.
- `README.md` and `README-EN.md` - add Rust build/run instructions.

## Complete Migration Roadmap

### Slice 1: Foundation and Low-Risk APIs

Implement:

- Rust project scaffold.
- Config defaults and logging.
- SQLite schema migration for existing GORM tables.
- Response envelope helpers.
- Password file parsing/writing.
- In-memory session middleware compatible with current auth behavior.
- Static file behavior that tolerates missing `dist/` and serves legacy frontend asset paths.
- Low-risk API parity for init state, auth, KV, and web links.
- Cluster schema/repository compatibility, with `/api/cluster` HTTP CRUD left as explicit 501 stubs until Go's install/world side effects and paginated response contract are migrated.
- Compatibility route manifest and explicit 501 responses for not-yet-migrated handlers.
- Build/Docker script rename to `dst-admin-rust`.

### Slice 2: Logs, SSE, and WebSocket

Implement:

- `/api/game/log/stream`
- `/ws`
- server log snapshot/download handlers
- tail-follow streaming with cancellation
- player log query/delete
- log parsing collector for join/leave/death/chat/regenerate/spawn events

### Slice 3: DST Process Control

Implement:

- process validation module for cluster/level identifiers
- Linux `screen` integration
- Windows process support where existing Go behavior is clear
- start/stop/status/update game
- system info endpoint and stream
- console command handlers
- broadcast/player operations
- rollback/regenerate/clean world

### Slice 4: Mods, Third-Party APIs, Map, and Backup Restore

Implement:

- Steam Workshop detail calls
- installed mod discovery
- `modinfo.lua` parse/write
- UGC upload/delete
- third-party DST server list and static proxy APIs
- map image generation
- backup upload/download/restore
- snapshot backup retention

### Slice 5: Scheduling and Auto Check

Implement:

- persisted cron tasks
- task execution strategies
- announcement scheduling
- level-down checks
- level-mod checks
- game-version update checks
- task status logs

### Slice 6: Final Release and Go Removal

Implement:

- full compatibility verification against manifest
- README migration notes
- release build validation
- Docker validation
- removal of Go backend files after Rust parity is verified

## Slice 1 Detailed Tasks

### Task 1: Cargo Project and Config Defaults

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/config.rs`
- Test: `tests/config_tests.rs`
- Modify: `.gitignore`

- [ ] **Step 1: Write the failing config tests**

Create `tests/config_tests.rs`:

```rust
use std::fs;

use dst_admin_rust::config::AppConfig;
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
fn missing_config_file_returns_contextual_error() {
    let dir = tempdir().unwrap();
    let err = AppConfig::from_file(dir.path().join("missing.yml")).unwrap_err();
    assert!(err.to_string().contains("missing.yml"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test config_tests
```

Expected: FAIL because the Rust crate and `AppConfig` do not exist.

- [ ] **Step 3: Implement the minimal project and config loader**

Create `Cargo.toml` with package name `dst-admin-rust` and library crate `dst_admin_rust`. The binary target is added later in Task 7 when `src/main.rs` exists; Cargo will use the package name `dst-admin-rust` for that binary. Create `src/lib.rs` exporting `pub mod config;`. Create `src/config.rs` with:

- `//!` module comment.
- `AppConfig`.
- `AutoUpdateModinfoConfig`.
- `AppConfig::from_file`.
- serde aliases matching Go YAML keys.
- defaulting for Go-compatible zero/missing values.
- `tracing::info!` after successful config load without logging secrets.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --test config_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add Cargo.toml src/lib.rs src/config.rs tests/config_tests.rs .gitignore
git commit -m "feat: scaffold rust config loader"
```

### Task 2: Logging, Error Type, and Response Envelope

**Files:**
- Create: `src/logging.rs`
- Create: `src/error.rs`
- Create: `src/response.rs`
- Modify: `src/lib.rs`
- Test: `tests/http_tests.rs`

- [ ] **Step 1: Write the failing response tests**

Create `tests/http_tests.rs` with:

```rust
use dst_admin_rust::response::{ApiResponse, LoginResponse};
use serde_json::json;

#[test]
fn success_response_matches_go_result_envelope() {
    let body = serde_json::to_value(ApiResponse::success(json!({"ok": true}))).unwrap();
    assert_eq!(body, json!({"code": 0, "msg": "", "data": {"ok": true}}));
}

#[test]
fn login_response_preserves_go_login_semantics() {
    let body = serde_json::to_value(LoginResponse::success(json!({"username": "admin"}))).unwrap();
    assert_eq!(
        body,
        json!({"code": 200, "msg": "Login success", "data": {"username": "admin"}})
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test http_tests
```

Expected: FAIL because `response` does not exist.

- [ ] **Step 3: Implement logging, error, and response modules**

Implement:

- `logging::init(log_path: impl AsRef<Path>)`.
- `AppError` with `thiserror`.
- `ApiResponse<T>` with `success`, `error`, and `empty_success`.
- `LoginResponse<T>` with `success` and `error`.
- `IntoResponse` mapping for `AppError`.
- comments on envelope differences between general Go `Result` and login `Response`.
- `tracing` setup that writes to stdout and `dst-admin-go.log`.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --test http_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/logging.rs src/error.rs src/response.rs src/lib.rs tests/http_tests.rs
git commit -m "feat: add rust logging and response envelopes"
```

### Task 3: SQLite Schema Migration and Repositories

**Files:**
- Create: `src/db.rs`
- Create: `src/models.rs`
- Create: `src/repositories/mod.rs`
- Create: `src/repositories/kv.rs`
- Create: `src/repositories/web_link.rs`
- Create: `src/repositories/cluster.rs`
- Modify: `src/lib.rs`
- Test: `tests/db_tests.rs`

- [ ] **Step 1: Write failing database tests**

Create `tests/db_tests.rs` with tests that:

- open an in-memory SQLite database.
- call `db::migrate`.
- assert GORM-compatible tables exist: `clusters`, `kvs`, `web_links`, `backup_snapshots`, `auto_checks`, `job_tasks`, `mod_infos`, `player_logs`, `spawns`, `connects`, `regenerates`, `announces`, `log_records`.
- insert/read/update/delete KV.
- insert/list/delete web links.
- insert/list/update/delete clusters.

Use exact repository APIs:

```rust
use dst_admin_rust::{
    db,
    models::{ClusterRecord, NewCluster, NewWebLink},
    repositories::{cluster::ClusterRepository, kv::KvRepository, web_link::WebLinkRepository},
};

#[tokio::test]
async fn migration_creates_go_compatible_tables() {
    let pool = db::connect_sqlite_memory().await.unwrap();
    db::migrate(&pool).await.unwrap();

    for table in [
        "clusters",
        "kvs",
        "web_links",
        "backup_snapshots",
        "auto_checks",
        "job_tasks",
        "mod_infos",
        "player_logs",
        "spawns",
        "connects",
        "regenerates",
        "announces",
        "log_records",
    ] {
        assert!(db::table_exists(&pool, table).await.unwrap(), "{table} missing");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test db_tests
```

Expected: FAIL because `db`, `models`, and repositories do not exist.

- [ ] **Step 3: Implement schema and repositories**

Implement explicit `CREATE TABLE IF NOT EXISTS` statements preserving:

- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `created_at DATETIME`
- `updated_at DATETIME`
- `deleted_at DATETIME`
- Go JSON field names in Rust DTO serde attributes.
- `clusters.cluster_name` unique index.

Implement repository methods:

- `KvRepository::get`
- `KvRepository::save`
- `WebLinkRepository::list`
- `WebLinkRepository::add`
- `WebLinkRepository::delete`
- `ClusterRepository::list`
- `ClusterRepository::create`
- `ClusterRepository::update`
- `ClusterRepository::delete`

Add `tracing::info!` for migration start/end and `tracing::warn!` for duplicate/invalid repository operations.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --test db_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/db.rs src/models.rs src/repositories tests/db_tests.rs src/lib.rs
git commit -m "feat: add sqlite schema and repositories"
```

### Task 4: Auth, Password File, and Session Middleware

**Files:**
- Create: `src/auth.rs`
- Create: `src/handlers/mod.rs`
- Create: `src/handlers/auth.rs`
- Modify: `src/lib.rs`
- Test: `tests/auth_tests.rs`

- [ ] **Step 1: Write failing auth tests**

Create `tests/auth_tests.rs`:

```rust
use std::fs;

use dst_admin_rust::auth::{PasswordFile, SessionStore, UserCredentials};
use tempfile::tempdir;

#[test]
fn parses_existing_password_file_format() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("password.txt");
    fs::write(
        &path,
        "username=admin\npassword=123456\ndisplayName=admin\nphotoURL=xxx\n",
    )
    .unwrap();

    let password_file = PasswordFile::read(&path).unwrap();

    assert_eq!(password_file.username, "admin");
    assert_eq!(password_file.password, "123456");
    assert_eq!(password_file.display_name, "admin");
    assert_eq!(password_file.photo_url, "xxx");
}

#[test]
fn session_store_creates_and_validates_sessions_without_exposing_values() {
    let store = SessionStore::default();
    let session_id = store.create_session("admin");

    assert!(store.validate(&session_id).is_some());
    assert!(store.validate("missing").is_none());
    assert_ne!(session_id, "admin");
}

#[test]
fn credentials_are_not_logged_or_serialized_with_password_after_login() {
    let mut creds = UserCredentials {
        username: "admin".to_string(),
        password: "123456".to_string(),
        session_id: None,
    };
    creds.clear_password();
    assert_eq!(creds.password, "");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test auth_tests
```

Expected: FAIL because `auth` does not exist.

- [ ] **Step 3: Implement password and sessions**

Implement:

- `PasswordFile::read`.
- `PasswordFile::write`.
- `SessionStore` backed by `Arc<RwLock<HashMap<String, SessionRecord>>>`.
- session creation with UUID.
- session cookie name `token`.
- whitelist paths matching Go middleware.
- `is_white_ip(remote_addr, whiteadminip)`.
- auth handlers for login/logout/user/password/update user.

Add logs for auth failures and direct login decisions, without logging passwords or session IDs.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --test auth_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/auth.rs src/handlers/mod.rs src/handlers/auth.rs src/lib.rs tests/auth_tests.rs
git commit -m "feat: add rust auth and sessions"
```

### Task 5: App Router, Static Assets, Init, KV, Web Links, and Cluster Stubs

**Files:**
- Create: `src/app.rs`
- Create: `src/handlers/init.rs`
- Create: `src/handlers/kv.rs`
- Create: `src/handlers/web_link.rs`
- Create: `src/handlers/static_files.rs`
- Modify: `src/lib.rs`
- Modify: `tests/http_tests.rs`

- [ ] **Step 1: Write failing HTTP integration tests**

Extend `tests/http_tests.rs` with tokio tests that:

- build an app with a temporary config root and in-memory SQLite.
- assert `/hello` returns `Hello! Dont starve together`.
- assert protected `/api/kv` returns 401 without a session.
- login with `password.txt`, retain `Set-Cookie`, then call `/api/kv`.
- save and read a KV value.
- add/list/delete a web link.
- call `/` when `dist/` is missing and assert the server does not panic.
- serve `/assets/*`, `/static/*`, `/favicon.ico`, and `/asset-manifest.json` from `dist/` when present.
- assert `/api/cluster` GET/POST/PUT/DELETE return explicit 501 stubs until Go side effects and response contracts are migrated.

Use `tower::ServiceExt`:

```rust
let response = app
    .oneshot(
        Request::builder()
            .method(Method::GET)
            .uri("/hello")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
assert_eq!(response.status(), StatusCode::OK);
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test http_tests
```

Expected: FAIL because app/router handlers do not exist.

- [ ] **Step 3: Implement app state and low-risk routes**

Implement:

- `AppState` with config, db pool, session store, root path.
- `build_router(AppState) -> Router`.
- auth middleware for `/api` routes except Go whitelist paths.
- `/hello`.
- `/api/init` GET checks whether `first` exists, matching Go's first-run intent.
- `/api/kv` GET/POST.
- `/api/web/link` GET/POST/DELETE.
- static fallback for `dist/index.html` when present and 404 when absent.
- static asset routes for the legacy frontend paths.
- `/api/cluster` GET/POST/PUT/DELETE as explicit 501 compatibility stubs. Do not mark them implemented in slice 1 because Go create performs DST install/world initialization side effects and Go list returns a paginated runtime VO that is not yet migrated.

Add logs for route validation failures, repository errors, and static asset availability.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --test http_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/app.rs src/handlers/init.rs src/handlers/kv.rs src/handlers/web_link.rs src/handlers/static_files.rs src/lib.rs tests/http_tests.rs
git commit -m "feat: add rust app router and low-risk apis"
```

### Task 6: Compatibility Route Manifest and Explicit 501 Handlers

**Files:**
- Create: `src/handlers/compat.rs`
- Modify: `src/app.rs`
- Modify: `src/handlers/mod.rs`
- Test: `tests/compat_manifest_tests.rs`

- [ ] **Step 1: Write failing route manifest tests**

Create `tests/compat_manifest_tests.rs` with a route manifest containing all current Go route method/path patterns. The test must assert every route is registered and, when not implemented in slice 1, returns HTTP 501 with a Go-compatible response envelope:

```rust
assert_eq!(body["code"], 501);
assert!(body["msg"].as_str().unwrap().contains("not migrated"));
```

Include streaming and file routes in the manifest, but mark them as `ExpectedKind::StreamingStub` for slice 1.

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --test compat_manifest_tests
```

Expected: FAIL because compatibility routes are not registered.

- [ ] **Step 3: Implement explicit compatibility routes**

Register not-yet-migrated routes from Go routers with clear handler names. Return:

- HTTP 501.
- JSON `{ "code": 501, "msg": "route not migrated to Rust yet: METHOD PATH", "data": {} }`.

Do not use a catch-all for `/api`; explicit routes prevent accidental compatibility gaps.

Add `tracing::warn!` for each compatibility stub hit.

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cargo test --test compat_manifest_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/handlers/compat.rs src/app.rs src/handlers/mod.rs tests/compat_manifest_tests.rs
git commit -m "feat: register rust compatibility route manifest"
```

### Task 7: Binary Entrypoint and Release Script Rename

**Files:**
- Create: `src/main.rs`
- Modify: `build_linux.sh`
- Modify: `build_window.sh`
- Modify: `Dockerfile`
- Modify: `docker-entrypoint.sh`
- Modify: `README.md`
- Modify: `README-EN.md`

- [ ] **Step 1: Write failing binary smoke test command**

Run:

```bash
cargo build --bin dst-admin-rust
```

Expected: FAIL because `src/main.rs` does not exist or startup wiring is incomplete.

- [ ] **Step 2: Implement binary startup**

Implement `src/main.rs`:

- initialize logging.
- load `config.yml`.
- open configured SQLite database.
- run migrations.
- build router.
- bind to `bindAddress:port`.
- log the effective address.

- [ ] **Step 3: Update build and Docker scripts**

Update:

- `build_linux.sh` to run `cargo build --release --bin dst-admin-rust` and copy/use `target/release/dst-admin-rust`.
- `build_window.sh` to build the Windows target when available.
- `Dockerfile` to `COPY dst-admin-rust /app/dst-admin-rust`.
- `docker-entrypoint.sh` to `exec ./dst-admin-rust`.
- README files with Rust build/run commands.

- [ ] **Step 4: Run build verification**

Run:

```bash
cargo build --bin dst-admin-rust
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/main.rs build_linux.sh build_window.sh Dockerfile docker-entrypoint.sh README.md README-EN.md
git commit -m "feat: add dst-admin-rust binary and release scripts"
```

### Task 8: Slice 1 Verification

**Files:**
- Modify only files needed to fix verification failures.

- [ ] **Step 1: Run formatting**

Run:

```bash
cargo fmt --all --check
```

Expected: PASS.

- [ ] **Step 2: Run clippy**

Run:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: PASS.

- [ ] **Step 3: Run full tests**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 4: Run release build**

Run:

```bash
cargo build --release --bin dst-admin-rust
```

Expected: PASS.

- [ ] **Step 5: Commit verification fixes**

If verification required fixes, commit:

```bash
git add .
git commit -m "chore: verify rust migration foundation"
```

If no fixes were needed, do not create an empty commit.

## Post-Slice 1 Handoff

After slice 1 is complete:

- Create `docs/superpowers/plans/2026-06-10-rust-migration-logs.md` for slice 2.
- Do not remove Go files yet.
- Do not claim full migration parity yet.
- Report which routes are fully migrated and which routes return explicit 501 compatibility responses.
