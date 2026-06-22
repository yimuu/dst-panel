# Rust Parity Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the remaining non-security Go/Rust parity gaps in the Rust 2024 `dst-admin-rust` backend.

**Architecture:** Keep the current Rust codebase as the migration target and reuse the existing fakeable `AppState` boundaries: `CommandRunner`, `HttpClient`, and `ProcessSnapshotProvider`. Route handlers remain thin; DST install, cluster runtime enrichment, online player querying, system metrics, and platform process collection move into focused game/process modules with structured tracing logs.

**Tech Stack:** Rust 2024, Axum 0.8, Tokio, SQLx SQLite, Reqwest-backed `HttpClient`, argv-based `CommandRunner`, `tracing`, integration tests under `tests/`.

---

## Scope

This plan excludes intentional security hardening differences from Go, including stricter WebSocket tail access, path traversal protection, symlink rejection, bounded log/static reads, and shell-free command execution. Those are accepted Rust behavior and should not be weakened for byte-for-byte Go compatibility.

This plan does include:

- Cluster creation side effects that Go performs.
- Cluster list runtime fields that Rust still returns as placeholders.
- Online player routes that currently return empty lists.
- Dashboard system metrics whose shape is compatible but whose values are incomplete.
- Windows process snapshot support, if the Rust binary is expected to keep Go's Windows deployment support.
- Final cleanup of stale migration scaffolding and comments after parity tasks pass.

## File Structure

- Modify `src/app.rs`: add a zero-delay test override for online player queries if needed by tests.
- Modify `src/game/mod.rs`: expose focused submodules and replace incomplete system metric helpers.
- Create `src/game/install.rs`: Go-compatible DST dedicated server install-on-create command construction.
- Create `src/game/cluster_runtime.rs`: process/lobby/cluster.ini based runtime enrichment for cluster list rows.
- Create `src/game/player_query.rs`: Go-compatible online player command construction and log parsing.
- Modify `src/handlers/cluster.rs`: call install and runtime enrichment instead of returning placeholders.
- Modify `src/handlers/player.rs`: call online player query service instead of returning empty vectors.
- Modify `src/handlers/game.rs`: use injected `state.process_snapshot_provider` for status routes.
- Modify `src/process/mod.rs`: add Windows snapshot collection and parser tests.
- Modify `tests/cluster_level_tests.rs`: update cluster create/list assertions and add install/runtime coverage.
- Modify `tests/game_process_tests.rs`: prove status uses injected process provider and improve system-info assertions.
- Modify `tests/game_lifecycle_tests.rs`: move or duplicate fake process provider helpers only when the target test needs them.
- Add `tests/player_query_tests.rs`: route and parser coverage for online players.

---

### Task 1: Use Injected Process Provider Everywhere Status Is Reported

**Files:**
- Modify: `src/handlers/game.rs`
- Test: `tests/game_process_tests.rs`

- [ ] **Step 1: Write the failing integration test**

Add a local fake provider to `tests/game_process_tests.rs` and a test that expects `/api/game/8level/status` to use injected snapshots.

```rust
use std::{collections::VecDeque, io, sync::{Arc, Mutex}};
use dst_admin_rust::process::{ProcessSnapshot, ProcessSnapshotProvider};

#[tokio::test]
async fn game_status_route_uses_injected_process_snapshot_provider() {
    let (app, dir) = test_router_with_processes(vec![ProcessSnapshot {
        pid: Some(4242),
        cpu_usage: "3.5".to_owned(),
        mem_usage: "1.2".to_owned(),
        virtual_size: "123456".to_owned(),
        resident_set_size: "7890".to_owned(),
        command: "./dontstarve_dedicated_server_nullrenderer -console -cluster ClusterInjectedStatus -shard Master".to_owned(),
    }])
    .await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterInjectedStatus");
    write_level_fixture(dir.path(), "ClusterInjectedStatus");

    let response = send(
        &app,
        Method::GET,
        "/api/game/8level/status",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["data"][0]["uuid"], "Master");
    assert_eq!(body["data"][0]["status"], true);
    assert_eq!(
        body["data"][0]["Ps"],
        json!({"cpuUage": "3.5", "memUage": "1.2", "VSZ": "123456", "RSS": "7890"})
    );
}

#[derive(Debug, Clone)]
struct FakeProcessSnapshotProvider {
    snapshots: Arc<Mutex<VecDeque<Vec<ProcessSnapshot>>>>,
}

impl FakeProcessSnapshotProvider {
    fn new(snapshots: Vec<Vec<ProcessSnapshot>>) -> Self {
        Self {
            snapshots: Arc::new(Mutex::new(snapshots.into())),
        }
    }
}

impl ProcessSnapshotProvider for FakeProcessSnapshotProvider {
    fn snapshots(&self) -> io::Result<Vec<ProcessSnapshot>> {
        Ok(self
            .snapshots
            .lock()
            .expect("fake snapshots poisoned")
            .front()
            .cloned()
            .unwrap_or_default())
    }
}

async fn test_router_with_processes(snapshots: Vec<ProcessSnapshot>) -> (Router, TempDir) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    let state = AppState::new_with_command_runner_and_process_snapshot_provider(
        test_config(),
        pool,
        SessionStore::new(),
        dir.path(),
        dst_admin_rust::command::FakeCommandRunner::default(),
        FakeProcessSnapshotProvider::new(vec![snapshots]),
    );
    (build_router(state), dir)
}
```

- [ ] **Step 2: Run the focused failing test**

Run:

```bash
cargo test --test game_process_tests game_status_route_uses_injected_process_snapshot_provider
```

Expected: FAIL because `status_handler` still calls `SystemProcessSnapshotProvider` directly.

- [ ] **Step 3: Implement provider injection in the handler**

Change `src/handlers/game.rs` so `status_handler` clones `state.process_snapshot_provider` before `spawn_blocking` and calls that provider.

```rust
pub(crate) async fn status_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<Vec<LevelStatusInfo>>>> {
    let root_path = state.root_path.clone();
    let process_provider = state.process_snapshot_provider.clone();
    let statuses = tokio::task::spawn_blocking(move || {
        let cluster_name =
            dst::current_cluster_name(&root_path).map_err(file_error("resolve cluster name"))?;
        let cluster_name = validate_safe_command_arg("cluster name", &cluster_name)
            .map_err(|error| AppError::bad_request(error.to_string()))?
            .into_string();
        let cluster_dir = dst::cluster_dir(&root_path, &cluster_name)
            .map_err(file_error("resolve cluster directory"))?;
        let worlds = level::list_existing_worlds_from_cluster_dir(&cluster_dir)?;
        let snapshots = match process_provider.snapshots() {
            Ok(snapshots) => snapshots,
            Err(error) => {
                tracing::warn!(
                    cluster_name,
                    error = %error,
                    "failed to collect process snapshots; reporting levels as stopped"
                );
                Vec::new()
            }
        };
        tracing::debug!(
            cluster_name,
            level_count = worlds.len(),
            process_count = snapshots.len(),
            "built DST level status response"
        );
        game::level_statuses_from_snapshots(&cluster_name, worlds, &snapshots)
            .map_err(|error| AppError::bad_request(error.to_string()))
    })
    .await
    .map_err(|error| {
        tracing::error!(error = %error, "status handler worker panicked or was cancelled");
        AppError::internal("collect level status")
    })??;

    Ok(Json(legacy_success(statuses)))
}
```

- [ ] **Step 4: Run focused and baseline tests**

Run:

```bash
cargo test --test game_process_tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/handlers/game.rs tests/game_process_tests.rs
git commit -m "fix: use injected process snapshots for status"
```

---

### Task 2: Add Go-Compatible DST Install-On-Cluster-Create

**Files:**
- Create: `src/game/install.rs`
- Modify: `src/game/mod.rs`
- Modify: `src/handlers/cluster.rs`
- Test: `tests/cluster_level_tests.rs`

- [ ] **Step 1: Update existing cluster create test to skip install intentionally**

In `cluster_routes_create_page_update_and_delete_without_starting_dst`, create the install directory before POST so the test exercises CRUD/file skeleton behavior without launching SteamCMD.

```rust
let install_dir = dir.path().join("server");
fs::create_dir_all(&install_dir).unwrap();

let created = send(
    &app,
    Method::POST,
    "/api/cluster",
    Some(json!({
        "clusterName": "ClusterApi",
        "description": "first shard",
        "steamcmd": "/opt/steamcmd",
        "force_install_dir": install_dir.display().to_string(),
        "backup": dir.path().join("backup").display().to_string(),
        "mod_download_path": dir.path().join("mods").display().to_string(),
        "uuid": "",
        "beta": 0,
        "bin": 64,
        "ugc_directory": "",
        "persistent_storage_root": "",
        "conf_dir": ""
    })),
    Some(&cookie),
)
.await;
```

- [ ] **Step 2: Write install command test**

Add a new test using `FakeCommandRunner` to prove missing `force_install_dir` triggers Go's SteamCMD app update command without shell interpolation.

```rust
#[tokio::test]
async fn cluster_create_installs_dst_when_force_install_dir_is_missing() {
    let runner = dst_admin_rust::command::FakeCommandRunner::new(vec![
        dst_admin_rust::command::CommandOutput::success(b"installed".to_vec(), Vec::new()),
    ]);
    let (app, dir, runner) = test_router_with_command_runner(runner).await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterInstallOnCreate");
    let steamcmd = dir.path().join("steamcmd");
    fs::create_dir_all(&steamcmd).unwrap();
    let install_dir = dir.path().join("dst-server");

    let response = send(
        &app,
        Method::POST,
        "/api/cluster",
        Some(json!({
            "clusterName": "ClusterInstallOnCreate",
            "description": "install",
            "steamcmd": steamcmd.display().to_string(),
            "force_install_dir": install_dir.display().to_string(),
            "backup": dir.path().join("backup").display().to_string(),
            "mod_download_path": dir.path().join("mods").display().to_string(),
            "uuid": "",
            "beta": 0,
            "bin": 64,
            "ugc_directory": "",
            "persistent_storage_root": "",
            "conf_dir": ""
        })),
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].program(), "./steamcmd.sh");
    assert_eq!(calls[0].current_dir().unwrap(), steamcmd.as_path());
    let expected_args = vec![
        "+login".to_owned(),
        "anonymous".to_owned(),
        "+force_install_dir".to_owned(),
        install_dir.display().to_string(),
        "+app_update".to_owned(),
        "343050".to_owned(),
        "validate".to_owned(),
        "+quit".to_owned(),
    ];
    assert_eq!(calls[0].args(), expected_args.as_slice());
    assert_ne!(calls[0].program(), "sh");
    assert_ne!(calls[0].program(), "bash");
}
```

Add helper:

```rust
async fn test_router_with_command_runner(
    runner: dst_admin_rust::command::FakeCommandRunner,
) -> (Router, TempDir, dst_admin_rust::command::FakeCommandRunner) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    let state = AppState::new_with_command_runner(
        test_config(),
        pool,
        SessionStore::new(),
        dir.path(),
        runner.clone(),
    );
    (build_router(state), dir, runner)
}
```

- [ ] **Step 3: Write rollback-on-install-failure test**

```rust
#[tokio::test]
async fn cluster_create_rolls_back_database_row_when_dst_install_fails() {
    let runner = dst_admin_rust::command::FakeCommandRunner::new(vec![
        dst_admin_rust::command::CommandOutput {
            status_code: Some(1),
            stdout: Vec::new(),
            stderr: b"failed".to_vec(),
            timed_out: false,
            stdout_truncated: false,
            stderr_truncated: false,
        },
    ]);
    let (app, dir, _runner) = test_router_with_command_runner(runner).await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterInstallRollback");
    let steamcmd = dir.path().join("steamcmd");
    fs::create_dir_all(&steamcmd).unwrap();

    let response = send(
        &app,
        Method::POST,
        "/api/cluster",
        Some(json!({
            "clusterName": "ClusterInstallRollback",
            "description": "rollback",
            "steamcmd": steamcmd.display().to_string(),
            "force_install_dir": dir.path().join("dst-server").display().to_string(),
            "backup": dir.path().join("backup").display().to_string(),
            "mod_download_path": dir.path().join("mods").display().to_string(),
            "uuid": "",
            "beta": 0,
            "bin": 64,
            "ugc_directory": "",
            "persistent_storage_root": "",
            "conf_dir": ""
        })),
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let listed = send(&app, Method::GET, "/api/cluster", None, Some(&cookie)).await;
    assert_eq!(response_json(listed).await["data"]["data"], json!([]));
}
```

- [ ] **Step 4: Run failing cluster tests**

Run:

```bash
cargo test --test cluster_level_tests cluster_create_installs_dst_when_force_install_dir_is_missing cluster_create_rolls_back_database_row_when_dst_install_fails
```

Expected: FAIL because create currently does not call SteamCMD.

- [ ] **Step 5: Implement `src/game/install.rs`**

```rust
//! Go-compatible DST dedicated server installation helpers.
//!
//! Cluster creation in Go installs app 343050 when the target install
//! directory does not exist. This module preserves that side effect through the
//! argv-based command boundary so request data never becomes a shell string.

use std::{path::{Path, PathBuf}, time::Duration};

use crate::{
    command::{CommandOutput, CommandRunner, CommandSpec},
    error::{AppError, AppResult},
};

const DST_APP_ID: &str = "343050";
const INSTALL_TIMEOUT: Duration = Duration::from_secs(60 * 60);

pub(crate) async fn install_dedicated_server_if_missing(
    runner: &dyn CommandRunner,
    cluster_name: &str,
    steam_cmd: &str,
    force_install_dir: &str,
) -> AppResult<()> {
    let steam_cmd = PathBuf::from(steam_cmd);
    let force_install_dir = PathBuf::from(force_install_dir);
    if force_install_dir.exists() {
        tracing::info!(
            cluster_name,
            force_install_dir = %force_install_dir.display(),
            "skipping DST install because force_install_dir already exists"
        );
        return Ok(());
    }

    let spec = install_spec(&steam_cmd, &force_install_dir)?;
    tracing::info!(
        cluster_name,
        steamcmd = %steam_cmd.display(),
        force_install_dir = %force_install_dir.display(),
        "installing DST dedicated server during cluster create"
    );
    let output = runner
        .run(spec)
        .await
        .map_err(|error| {
            tracing::warn!(cluster_name, error = %error, "DST install command failed before exit");
            AppError::internal("install dst dedicated server")
        })?;
    if command_succeeded(&output) {
        tracing::info!(
            cluster_name,
            status_code = ?output.status_code,
            stdout_len = output.stdout.len(),
            stderr_len = output.stderr.len(),
            "DST install command completed"
        );
        return Ok(());
    }

    tracing::warn!(
        cluster_name,
        status_code = ?output.status_code,
        timed_out = output.timed_out,
        stdout_len = output.stdout.len(),
        stderr_len = output.stderr.len(),
        "DST install command exited unsuccessfully"
    );
    Err(AppError::internal("install dst dedicated server"))
}

fn install_spec(steam_cmd: &Path, force_install_dir: &Path) -> AppResult<CommandSpec> {
    let force_install_dir = force_install_dir
        .to_str()
        .ok_or_else(|| AppError::bad_request("force_install_dir must be valid UTF-8"))?;
    Ok(CommandSpec::new("./steamcmd.sh")
        .with_current_dir(steam_cmd)
        .extend_args([
            "+login",
            "anonymous",
            "+force_install_dir",
            force_install_dir,
            "+app_update",
            DST_APP_ID,
            "validate",
            "+quit",
        ])
        .with_timeout(INSTALL_TIMEOUT))
}

fn command_succeeded(output: &CommandOutput) -> bool {
    !output.timed_out && output.status_code == Some(0)
}
```

- [ ] **Step 6: Wire install into cluster create rollback block**

In `src/game/mod.rs`:

```rust
pub(crate) mod install;
```

In `src/handlers/cluster.rs`, call install after the row is created and before skeleton creation, while preserving rollback on any side-effect failure.

```rust
let install_result = game::install::install_dedicated_server_if_missing(
    state.command_runner.as_ref(),
    cluster_name.as_str(),
    &created.steam_cmd,
    &created.force_install_dir,
)
.await;
if let Err(error) = install_result {
    tracing::error!(
        id = created.id,
        cluster_name = cluster_name.as_str(),
        error = %error,
        "rolling back cluster record after DST installation failed"
    );
    if let Err(delete_error) = repository.hard_delete_for_rollback(created.id).await {
        tracing::error!(
            id = created.id,
            error = %delete_error,
            "failed to remove cluster row after DST installation failure"
        );
    }
    return Err(error);
}
```

Also update the module comment in `cluster.rs` so it no longer says Rust stops before install.

- [ ] **Step 7: Run cluster tests**

Run:

```bash
cargo test --test cluster_level_tests
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src/game/install.rs src/game/mod.rs src/handlers/cluster.rs tests/cluster_level_tests.rs
git commit -m "feat: install DST during cluster create"
```

---

### Task 3: Populate Cluster List Runtime Fields

**Files:**
- Create: `src/game/cluster_runtime.rs`
- Modify: `src/game/mod.rs`
- Modify: `src/handlers/cluster.rs`
- Test: `tests/cluster_level_tests.rs`

- [ ] **Step 1: Write process runtime field test**

```rust
#[tokio::test]
async fn cluster_list_reports_master_and_caves_status_from_process_provider() {
    let snapshots = vec![
        ProcessSnapshot {
            pid: Some(1001),
            cpu_usage: "0.1".to_owned(),
            mem_usage: "0.2".to_owned(),
            virtual_size: "100".to_owned(),
            resident_set_size: "20".to_owned(),
            command: "./dontstarve_dedicated_server_nullrenderer -cluster ClusterRuntime -shard Master".to_owned(),
        },
        ProcessSnapshot {
            pid: Some(1002),
            cpu_usage: "0.3".to_owned(),
            mem_usage: "0.4".to_owned(),
            virtual_size: "200".to_owned(),
            resident_set_size: "40".to_owned(),
            command: "./dontstarve_dedicated_server_nullrenderer -cluster ClusterRuntime -shard Caves".to_owned(),
        },
    ];
    let (app, dir) = test_router_with_processes_and_http(snapshots, Vec::new()).await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterRuntime");
    seed_cluster_row(&app, &cookie, dir.path(), "ClusterRuntime").await;

    let listed = send(&app, Method::GET, "/api/cluster", None, Some(&cookie)).await;
    let body = response_json(listed).await;

    assert_eq!(body["data"]["data"][0]["master"], true);
    assert_eq!(body["data"]["data"][0]["caves"], true);
}
```

- [ ] **Step 2: Write lobby runtime field test**

Use a fake HTTP response matching Go's escaped-string upstream format and verify row fields.

```rust
#[tokio::test]
async fn cluster_list_enriches_lobby_fields_from_go_server_list_response() {
    let response_body = r#""{\"success\":true,\"successinfo\":{\"data\":[[\"row-abc\",0,0,0,0,3,8,0,\"survival\",4,\"我的饥荒服务世界\",1,0,0,\"autumn\",0,0,0,0,0,\"ap-east\"]]}}""#;
    let http = vec![dst_admin_rust::http_client::HttpResponse::new(200)
        .header("content-type", "application/json")
        .body(response_body)];
    let (app, dir) = test_router_with_processes_and_http(Vec::new(), http).await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterLobby");
    let cluster_dir = dir.path().join(".klei/DoNotStarveTogether/ClusterLobby");
    fs::create_dir_all(&cluster_dir).unwrap();
    fs::write(
        cluster_dir.join("cluster.ini"),
        "[GAMEPLAY]\ngame_mode = survival\nmax_players = 8\n[NETWORK]\ncluster_name = 我的饥荒服务世界\ncluster_password = secret\n",
    )
    .unwrap();
    seed_cluster_row(&app, &cookie, dir.path(), "ClusterLobby").await;

    let listed = send(&app, Method::GET, "/api/cluster", None, Some(&cookie)).await;
    let item = response_json(listed).await["data"]["data"][0].clone();

    assert_eq!(item["rowId"], "row-abc");
    assert_eq!(item["connected"], 3);
    assert_eq!(item["maxConnections"], 8);
    assert_eq!(item["mode"], "survival");
    assert_eq!(item["mods"], 4);
    assert_eq!(item["season"], "autumn");
    assert_eq!(item["region"], "ap-east");
    assert_eq!(item["password"], "");
}
```

Add concrete helpers used by both runtime tests:

```rust
#[derive(Debug, Clone)]
struct FakeProcessSnapshotProvider {
    snapshots: Arc<Mutex<VecDeque<Vec<ProcessSnapshot>>>>,
}

impl FakeProcessSnapshotProvider {
    fn new(snapshots: Vec<Vec<ProcessSnapshot>>) -> Self {
        Self {
            snapshots: Arc::new(Mutex::new(snapshots.into())),
        }
    }
}

impl ProcessSnapshotProvider for FakeProcessSnapshotProvider {
    fn snapshots(&self) -> io::Result<Vec<ProcessSnapshot>> {
        Ok(self
            .snapshots
            .lock()
            .expect("fake snapshots poisoned")
            .front()
            .cloned()
            .unwrap_or_default())
    }
}

async fn test_router_with_processes_and_http(
    snapshots: Vec<ProcessSnapshot>,
    http_responses: Vec<dst_admin_rust::http_client::HttpResponse>,
) -> (Router, TempDir) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    let state = AppState::new_with_command_runner_and_http_client(
        test_config(),
        pool,
        SessionStore::new(),
        dir.path(),
        dst_admin_rust::command::FakeCommandRunner::default(),
        dst_admin_rust::http_client::FakeHttpClient::new(http_responses),
        FakeProcessSnapshotProvider::new(vec![snapshots]),
    );
    (build_router(state), dir)
}

async fn seed_cluster_row(app: &Router, cookie: &str, root: &Path, cluster_name: &str) {
    let install_dir = root.join(format!("{cluster_name}-server"));
    fs::create_dir_all(&install_dir).unwrap();
    let created = send(
        app,
        Method::POST,
        "/api/cluster",
        Some(json!({
            "clusterName": cluster_name,
            "description": "seed",
            "steamcmd": root.join("steamcmd").display().to_string(),
            "force_install_dir": install_dir.display().to_string(),
            "backup": root.join("backup").display().to_string(),
            "mod_download_path": root.join("mods").display().to_string(),
            "uuid": "",
            "beta": 0,
            "bin": 64,
            "ugc_directory": "",
            "persistent_storage_root": "",
            "conf_dir": ""
        })),
        Some(cookie),
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);
}
```

- [ ] **Step 3: Run the failing focused tests**

Run:

```bash
cargo test --test cluster_level_tests cluster_list_reports_master_and_caves_status_from_process_provider cluster_list_enriches_lobby_fields_from_go_server_list_response
```

Expected: FAIL because the handler still returns default runtime fields.

- [ ] **Step 4: Implement `src/game/cluster_runtime.rs`**

```rust
//! Runtime enrichment for Go-compatible cluster list rows.
//!
//! Go combines DB rows, local process checks, `cluster.ini`, and a public lobby
//! lookup. The Rust handler keeps that behavior behind fakeable process and
//! HTTP adapters so tests do not depend on real DST processes or network access.

use std::{io, path::Path};

use serde_json::Value;

use crate::{
    dst::{self, cluster_ini::ClusterIni},
    http_client::{HttpClient, HttpRequest},
    models::ClusterRecord,
    process::{self, ProcessSnapshotProvider},
};

const SERVER_LIST_URL: &str = "https://dst.liuyh.com/index/serverlist/getserverlist.html";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ClusterRuntimeInfo {
    pub(crate) master: bool,
    pub(crate) caves: bool,
    pub(crate) row_id: String,
    pub(crate) connected: i64,
    pub(crate) max_connections: i64,
    pub(crate) mode: String,
    pub(crate) mods: i64,
    pub(crate) season: String,
    pub(crate) region: String,
}

pub(crate) async fn collect_for_cluster(
    root: &Path,
    record: &ClusterRecord,
    process_provider: &dyn ProcessSnapshotProvider,
    http_client: &dyn HttpClient,
) -> ClusterRuntimeInfo {
    let snapshots = match process_provider.snapshots() {
        Ok(snapshots) => snapshots,
        Err(error) => {
            tracing::warn!(
                cluster_name = %record.cluster_name,
                error = %error,
                "failed to collect process snapshots for cluster list"
            );
            Vec::new()
        }
    };
    let mut info = ClusterRuntimeInfo {
        master: process::first_level_process(&snapshots, &record.cluster_name, "Master").is_some(),
        caves: process::first_level_process(&snapshots, &record.cluster_name, "Caves").is_some(),
        ..ClusterRuntimeInfo::default()
    };

    let Some(cluster_ini) = read_cluster_ini(root, &record.cluster_name) else {
        return info;
    };
    if let Some(lobby) = fetch_matching_lobby(http_client, &cluster_ini).await {
        info.row_id = lobby.row_id;
        info.connected = lobby.connected;
        info.max_connections = lobby.max_connections;
        info.mode = lobby.mode;
        info.mods = lobby.mods;
        info.season = lobby.season;
        info.region = lobby.region;
    }
    info
}

fn read_cluster_ini(root: &Path, cluster_name: &str) -> Option<ClusterIni> {
    let cluster_dir = dst::cluster_dir(root, cluster_name).ok()?;
    let contents = dst::safe_read_cluster_file_to_string(&cluster_dir, "cluster.ini").ok()??;
    Some(ClusterIni::from_contents(&contents))
}

async fn fetch_matching_lobby(
    http_client: &dyn HttpClient,
    cluster_ini: &ClusterIni,
) -> Option<LobbyInfo> {
    let body = serde_json::json!({
        "page": 1,
        "paginate": 10,
        "sort_type": "name",
        "sort_way": 1,
        "search_type": 1,
        "search_content": cluster_ini.cluster_name,
        "mod": 1
    })
    .to_string();
    let request = HttpRequest::new("POST", SERVER_LIST_URL)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Content-Type", "application/json")
        .body(body);
    let response = http_client.send(request).await.ok()?;
    if response.status != 200 {
        tracing::warn!(status = response.status, "cluster lobby lookup returned non-200 status");
        return None;
    }
    let lobbies = parse_lobby_response(&response.body).ok()?;
    let has_password = if cluster_ini.cluster_password.is_empty() { 0 } else { 1 };
    lobbies.into_iter().find(|lobby| {
        lobby.name == cluster_ini.cluster_name
            && lobby.max_connections == cluster_ini.max_players as i64
            && lobby.mode == cluster_ini.game_mode
            && lobby.password == has_password
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LobbyInfo {
    row_id: String,
    connected: i64,
    max_connections: i64,
    mode: String,
    mods: i64,
    name: String,
    password: i64,
    season: String,
    region: String,
}

fn parse_lobby_response(bytes: &[u8]) -> io::Result<Vec<LobbyInfo>> {
    let value: Value = serde_json::from_slice(bytes).map_err(io::Error::other)?;
    let value = match value {
        Value::String(inner) => serde_json::from_str::<Value>(&inner).map_err(io::Error::other)?,
        value => value,
    };
    if value.get("success").and_then(Value::as_bool) != Some(true) {
        return Ok(Vec::new());
    }
    let rows = value
        .pointer("/successinfo/data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(rows
        .into_iter()
        .filter_map(|row| parse_lobby_row(row.as_array()?))
        .collect())
}

fn parse_lobby_row(row: &[Value]) -> Option<LobbyInfo> {
    Some(LobbyInfo {
        row_id: row.first()?.as_str()?.to_owned(),
        connected: row.get(5)?.as_i64()?,
        max_connections: row.get(6)?.as_i64()?,
        mode: row.get(8)?.as_str()?.to_owned(),
        mods: row.get(9)?.as_i64()?,
        name: row.get(10)?.as_str()?.to_owned(),
        password: row.get(11)?.as_i64()?,
        season: row.get(14)?.as_str()?.to_owned(),
        region: row.get(20)?.as_str()?.to_owned(),
    })
}
```

- [ ] **Step 5: Wire runtime info into `ClusterVo`**

Replace `impl From<ClusterRecord> for ClusterVo` with a constructor that accepts runtime info.

```rust
impl ClusterVo {
    fn from_record_and_runtime(record: ClusterRecord, runtime: game::cluster_runtime::ClusterRuntimeInfo) -> Self {
        Self {
            id: record.id,
            created_at: record.created_at,
            updated_at: record.updated_at,
            cluster_name: record.cluster_name,
            description: record.description,
            steam_cmd: record.steam_cmd,
            force_install_dir: record.force_install_dir,
            backup: record.backup,
            mod_download_path: record.mod_download_path,
            uuid: record.uuid,
            beta: record.beta,
            master: runtime.master,
            caves: runtime.caves,
            row_id: runtime.row_id,
            connected: runtime.connected,
            max_connections: runtime.max_connections,
            mode: runtime.mode,
            mods: runtime.mods,
            season: runtime.season,
            password: String::new(),
            region: runtime.region,
        }
    }
}
```

In `list_handler`, collect records first, then enrich each record:

```rust
let mut data = Vec::with_capacity(clusters.len());
for cluster in clusters {
    let runtime = game::cluster_runtime::collect_for_cluster(
        &state.root_path,
        &cluster,
        state.process_snapshot_provider.as_ref(),
        state.http_client.as_ref(),
    )
    .await;
    data.push(ClusterVo::from_record_and_runtime(cluster, runtime));
}
```

- [ ] **Step 6: Run tests**

Run:

```bash
cargo test --test cluster_level_tests
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/game/cluster_runtime.rs src/game/mod.rs src/handlers/cluster.rs tests/cluster_level_tests.rs
git commit -m "feat: enrich cluster list runtime fields"
```

---

### Task 4: Implement Online Player Query Routes

**Files:**
- Create: `src/game/player_query.rs`
- Modify: `src/game/mod.rs`
- Modify: `src/app.rs`
- Modify: `src/handlers/player.rs`
- Test: `tests/player_query_tests.rs`

- [ ] **Step 1: Add route tests**

Create `tests/player_query_tests.rs` with one parser-backed route test and one stopped-level test.

```rust
#[tokio::test]
async fn online_players_route_sends_go_lua_command_and_parses_recent_log_lines() {
    let runner = FakeCommandRunner::new(vec![CommandOutput::success(Vec::new(), Vec::new())]);
    let snapshots = vec![ProcessSnapshot {
        pid: Some(2001),
        cpu_usage: "0.0".to_owned(),
        mem_usage: "0.1".to_owned(),
        virtual_size: "10".to_owned(),
        resident_set_size: "20".to_owned(),
        command: "./dontstarve_dedicated_server_nullrenderer -cluster ClusterPlayers -shard Master".to_owned(),
    }];
    let marker = "1700000000";
    let (app, dir, runner) =
        test_router_with_runner_processes_zero_delay_and_marker(runner, snapshots, marker).await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterPlayers");
    let cluster_dir = dir.path().join(".klei/DoNotStarveTogether/ClusterPlayers");
    fs::create_dir_all(cluster_dir.join("Master")).unwrap();
    fs::write(
        cluster_dir.join("Master/server_log.txt"),
        format!(
            "[00:00:00]: player: {{[{marker}] [1] [12] [KU_abc123] [Alice] [wilson]}}\n"
        ),
    )
    .unwrap();

    let response = send(
        &app,
        Method::GET,
        "/api/game/8level/players?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(
        body["data"],
        json!([{"key":"1","day":"12","name":"Alice","kuId":"KU_abc123","role":"wilson"}])
    );
    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].program(), "screen");
    assert_eq!(calls[0].args()[0], "-S");
    assert_eq!(calls[0].args()[1], "DST_8level_ClusterPlayers_Master");
    assert!(calls[0].args()[6].contains("AllPlayers"));
}

#[tokio::test]
async fn online_players_route_returns_empty_without_sending_command_when_level_is_stopped() {
    let runner = FakeCommandRunner::default();
    let (app, dir, runner) =
        test_router_with_runner_processes_zero_delay_and_marker(runner, Vec::new(), "1700000000").await;
    let cookie = login(&app).await;
    write_dst_config(dir.path(), "ClusterPlayersStopped");

    let response = send(
        &app,
        Method::GET,
        "/api/game/8level/players?levelName=Master",
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await["data"], json!([]));
    assert!(runner.calls().is_empty());
}
```

Add concrete helpers for this test file:

```rust
#[derive(Debug, Clone)]
struct FakeProcessSnapshotProvider {
    snapshots: Arc<Mutex<VecDeque<Vec<ProcessSnapshot>>>>,
}

impl FakeProcessSnapshotProvider {
    fn new(snapshots: Vec<Vec<ProcessSnapshot>>) -> Self {
        Self {
            snapshots: Arc::new(Mutex::new(snapshots.into())),
        }
    }
}

impl ProcessSnapshotProvider for FakeProcessSnapshotProvider {
    fn snapshots(&self) -> io::Result<Vec<ProcessSnapshot>> {
        Ok(self
            .snapshots
            .lock()
            .expect("fake snapshots poisoned")
            .front()
            .cloned()
            .unwrap_or_default())
    }
}

async fn test_router_with_runner_processes_zero_delay_and_marker(
    runner: FakeCommandRunner,
    snapshots: Vec<ProcessSnapshot>,
    marker: &str,
) -> (Router, TempDir, FakeCommandRunner) {
    let dir = tempdir().unwrap();
    write_password_file(dir.path());
    let pool = connect_sqlite_memory().await.unwrap();
    migrate(&pool).await.unwrap();
    let state = AppState::new_with_command_runner_and_process_snapshot_provider(
        test_config(),
        pool,
        SessionStore::new(),
        dir.path(),
        runner.clone(),
        FakeProcessSnapshotProvider::new(vec![snapshots]),
    )
    .with_player_query_delay(Duration::ZERO)
    .with_player_query_marker_override(marker);
    (build_router(state), dir, runner)
}
```

- [ ] **Step 2: Run failing player tests**

Run:

```bash
cargo test --test player_query_tests
```

Expected: FAIL because the routes currently return empty data and send no command.

- [ ] **Step 3: Add player delay and marker overrides to `AppState`**

In `src/app.rs`, add:

```rust
/// Delay after sending the Go-compatible online-player Lua command before reading logs.
pub player_query_delay: Duration,
/// Optional deterministic marker used by online-player tests.
pub player_query_marker_override: Option<String>,
```

Initialize it in `AppState::new_with_command_runner_and_http_client`:

```rust
player_query_delay: Duration::from_secs(1),
player_query_marker_override: None,
```

Add test builders:

```rust
pub fn with_player_query_delay(mut self, player_query_delay: Duration) -> Self {
    self.player_query_delay = player_query_delay;
    self
}

pub fn with_player_query_marker_override(mut self, marker: impl Into<String>) -> Self {
    self.player_query_marker_override = Some(marker.into());
    self
}
```

- [ ] **Step 4: Implement `src/game/player_query.rs`**

```rust
//! Online player query implementation compatible with Go's log-scraping flow.
//!
//! Go sends a Lua print command into a DST screen session, waits briefly, then
//! parses recent `server_log.txt` lines containing a timestamp marker. Rust
//! preserves the external behavior while constructing screen commands through
//! `CommandRunner` and parsing logs with bounded, testable helpers.

use std::{collections::HashMap, path::Path, time::{Duration, SystemTime, UNIX_EPOCH}};

use serde::Serialize;

use crate::{
    command::CommandRunner,
    dst,
    error::{AppError, AppResult},
    game::console,
    logs,
    process::{self, ProcessSnapshotProvider},
    validation::validate_level_name,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PlayerVo {
    pub(crate) key: String,
    pub(crate) day: String,
    pub(crate) name: String,
    #[serde(rename = "kuId")]
    pub(crate) ku_id: String,
    pub(crate) role: String,
}

pub(crate) async fn query_online_players(
    root: &Path,
    runner: &dyn CommandRunner,
    process_provider: &dyn ProcessSnapshotProvider,
    level_name: &str,
    delay: Duration,
    marker_override: Option<&str>,
) -> AppResult<Vec<PlayerVo>> {
    let cluster_name = dst::current_cluster_name(root).map_err(file_error("resolve cluster"))?;
    let level = validate_level_name(level_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?
        .into_string();
    let status_level = if level == "#ALL_LEVEL" { "Master" } else { level.as_str() };
    if !level_running(process_provider, &cluster_name, status_level) {
        tracing::debug!(cluster_name, level_name = status_level, "online player query skipped because level is stopped");
        return Ok(Vec::new());
    }

    let marker = marker_override
        .map(ToOwned::to_owned)
        .unwrap_or_else(current_marker);
    let command_level = if level == "#ALL_LEVEL" { "Master" } else { level.as_str() };
    let command = player_lua_command(&marker, level == "#ALL_LEVEL");
    console::send_level_command(runner, &cluster_name, command_level, &command).await?;
    if !delay.is_zero() {
        tokio::time::sleep(delay).await;
    }

    let cluster_dir = dst::cluster_dir(root, &cluster_name).map_err(file_error("resolve cluster directory"))?;
    let log_path = cluster_dir.join(command_level).join("server_log.txt");
    let mut file = std::fs::File::open(&log_path).map_err(file_error("open player log"))?;
    let lines = logs::recent_lines_from_file(&mut file, 150).map_err(|error| {
        tracing::warn!(
            cluster_name,
            level_name = command_level,
            error = %error,
            "failed to read recent player log lines"
        );
        AppError::internal("read player log")
    })?;
    let players = parse_player_lines(lines.iter().map(String::as_str), &marker);
    tracing::info!(
        cluster_name,
        level_name = command_level,
        player_count = players.len(),
        "queried online players"
    );
    Ok(players)
}

fn level_running(
    provider: &dyn ProcessSnapshotProvider,
    cluster_name: &str,
    level_name: &str,
) -> bool {
    provider
        .snapshots()
        .ok()
        .and_then(|snapshots| process::first_level_process(&snapshots, cluster_name, level_name).cloned())
        .is_some()
}

fn current_marker() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn player_lua_command(marker: &str, all_levels: bool) -> String {
    if all_levels {
        format!(
            "for i, v in ipairs(TheNet:GetClientTable()) do  print(string.format(\\\"player: {{[%s] [%d] [%s] [%s] [%s] [%s]}} \\\", '{marker}', i-1, string.format('%03d', v.playerage), v.userid, v.name, v.prefab)) end"
        )
    } else {
        format!(
            "for i, v in ipairs(AllPlayers) do print(string.format(\\\"player: {{[%d] [%d] [%d] [%s] [%s] [%s]}} \\\", {marker}, i, v.components.age:GetAgeInDays(), v.userid, v.name, v.prefab)) end"
        )
    }
}

pub(crate) fn parse_player_lines<'a>(
    lines: impl IntoIterator<Item = &'a str>,
    marker: &str,
) -> Vec<PlayerVo> {
    let mut unique = HashMap::<String, PlayerVo>::new();
    for line in lines {
        if !line.contains(marker) || !line.contains("KU") || line.contains("Host") {
            continue;
        }
        if let Some(player) = parse_player_line(line) {
            unique.insert(player.ku_id.clone(), player);
        }
    }
    unique.into_values().collect()
}

fn parse_player_line(line: &str) -> Option<PlayerVo> {
    let start = line.find('{')?;
    let end = line[start..].find('}')? + start;
    let mut values = Vec::new();
    let mut rest = &line[start + 1..end];
    while let Some(open) = rest.find('[') {
        let after_open = &rest[open + 1..];
        let close = after_open.find(']')?;
        values.push(after_open[..close].to_owned());
        rest = &after_open[close + 1..];
    }
    if values.len() < 6 {
        return None;
    }
    Some(PlayerVo {
        key: values[1].clone(),
        day: values[2].clone(),
        ku_id: values[3].clone(),
        name: values[4].clone(),
        role: values[5].clone(),
    })
}

fn file_error(operation: &'static str) -> impl FnOnce(std::io::Error) -> AppError + Copy {
    move |error| {
        tracing::warn!(operation, error = %error, "online player operation failed");
        AppError::internal(operation)
    }
}
```

- [ ] **Step 5: Wire player routes**

In `src/game/mod.rs`:

```rust
pub(crate) mod player_query;
```

In `src/handlers/player.rs`, remove the local `PlayerVo` and call the service:

```rust
use crate::game::player_query::PlayerVo;

pub(crate) async fn online_players_handler(
    State(state): State<AppState>,
    Query(query): Query<PlayersQuery>,
) -> AppResult<Json<LoginResponse<Vec<PlayerVo>>>> {
    let level_name = query.level_name.as_deref().unwrap_or("Master");
    if !level_name.is_empty() {
        validate_level_name(level_name)
            .map_err(|error| AppError::bad_request(error.to_string()))?;
    }
    let players = game::player_query::query_online_players(
        &state.root_path,
        state.command_runner.as_ref(),
        state.process_snapshot_provider.as_ref(),
        level_name,
        state.player_query_delay,
        state.player_query_marker_override.as_deref(),
    )
    .await?;
    Ok(Json(legacy_success(players)))
}

pub(crate) async fn all_online_players_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LoginResponse<Vec<PlayerVo>>>> {
    let players = game::player_query::query_online_players(
        &state.root_path,
        state.command_runner.as_ref(),
        state.process_snapshot_provider.as_ref(),
        "#ALL_LEVEL",
        state.player_query_delay,
        state.player_query_marker_override.as_deref(),
    )
    .await?;
    Ok(Json(legacy_success(players)))
}
```

- [ ] **Step 6: Run player tests**

Run:

```bash
cargo test --test player_query_tests
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/app.rs src/game/player_query.rs src/game/mod.rs src/handlers/player.rs tests/player_query_tests.rs
git commit -m "feat: query online players from DST logs"
```

---

### Task 5: Improve System Metrics Parity With Go `gopsutil`

**Files:**
- Modify: `src/game/mod.rs`
- Test: `tests/game_process_tests.rs`

- [ ] **Step 1: Add pure metric parser tests**

Add tests for Linux-style `/proc/stat`, `/proc/meminfo`, and `/proc/mounts` parsing helpers. These keep CI deterministic while production reads real system files.

```rust
#[test]
fn cpu_stat_parser_reports_aggregate_and_per_core_percentages() {
    let previous = "cpu  100 0 100 800 0 0 0 0 0 0\ncpu0 50 0 50 400 0 0 0 0 0 0\ncpu1 50 0 50 400 0 0 0 0 0 0\n";
    let current = "cpu  150 0 150 900 0 0 0 0 0 0\ncpu0 75 0 75 450 0 0 0 0 0 0\ncpu1 75 0 75 450 0 0 0 0 0 0\n";
    let info = dst_admin_rust::game::cpu_info_from_proc_stat_pair(previous, current, 2);
    let value = serde_json::to_value(info).unwrap();

    assert_eq!(value["cores"], 2);
    assert_eq!(value["cpuPercent"].as_array().unwrap().len(), 2);
    assert!(value["cpuUsedPercent"].as_f64().unwrap() > 0.0);
    assert_eq!(
        value["cpuUsed"].as_f64().unwrap(),
        value["cpuUsedPercent"].as_f64().unwrap() * 0.01 * 2.0
    );
}

#[test]
fn host_platform_parser_reads_os_release_id() {
    let platform = dst_admin_rust::game::platform_from_os_release("NAME=\"Ubuntu\"\nID=ubuntu\n");
    assert_eq!(platform, "ubuntu");
}
```

- [ ] **Step 2: Run failing parser tests**

Run:

```bash
cargo test --test game_process_tests cpu_stat_parser_reports_aggregate_and_per_core_percentages host_platform_parser_reads_os_release_id
```

Expected: FAIL because helper functions are not exported yet.

- [ ] **Step 3: Implement CPU and host platform helpers**

Add public-for-tests helpers to `src/game/mod.rs`.

```rust
pub fn platform_from_os_release(contents: &str) -> String {
    contents
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key == "ID").then(|| value.trim_matches('"').to_owned())
        })
        .unwrap_or_default()
}

pub fn cpu_info_from_proc_stat_pair(previous: &str, current: &str, cores: i64) -> CpuInfo {
    let previous_rows = parse_proc_stat_cpu_rows(previous);
    let current_rows = parse_proc_stat_cpu_rows(current);
    let aggregate = usage_between(previous_rows.first(), current_rows.first());
    let cpu_percent = current_rows
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(index, row)| usage_between(previous_rows.get(index), Some(row)))
        .collect::<Vec<_>>();
    CpuInfo {
        cores,
        cpu_percent,
        cpu_used_percent: aggregate.unwrap_or_default(),
        cpu_used: aggregate.unwrap_or_default() * 0.01 * cores as f64,
    }
}
```

The parser should treat `idle + iowait` as idle time and all columns as total time, matching the common `gopsutil` calculation.

- [ ] **Step 4: Use the helpers in production collection**

Update `collect_host_info()` on Linux to read `/etc/os-release` for `platform`, and update `collect_cpu_info()` on Linux to sample `/proc/stat` twice with a short sleep.

```rust
#[cfg(target_os = "linux")]
fn collect_cpu_info() -> CpuInfo {
    let cores = std::thread::available_parallelism()
        .map(|count| i64::try_from(count.get()).unwrap_or(i64::MAX))
        .unwrap_or_default();
    let Ok(previous) = fs::read_to_string("/proc/stat") else {
        return zero_cpu_info(cores);
    };
    std::thread::sleep(std::time::Duration::from_millis(100));
    let Ok(current) = fs::read_to_string("/proc/stat") else {
        return zero_cpu_info(cores);
    };
    cpu_info_from_proc_stat_pair(&previous, &current, cores)
}
```

Keep `panel_cpu_usage: 0.0` because Go's panel CPU code is commented out and returns the zero value.

- [ ] **Step 5: Broaden system-info route assertions**

In `system_info_route_returns_go_compatible_dashboard_shape`, keep shape assertions and add Linux-only value assertions:

```rust
#[cfg(target_os = "linux")]
{
    assert!(body["data"]["cpu"]["cores"].as_i64().unwrap() >= 1);
    assert!(body["data"]["cpu"]["cpuPercent"].as_array().unwrap().len() >= 1);
    assert!(body["data"]["mem"]["total"].as_u64().unwrap() > 0);
}
```

- [ ] **Step 6: Run tests**

Run:

```bash
cargo test --test game_process_tests
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/game/mod.rs tests/game_process_tests.rs
git commit -m "feat: improve system info metrics"
```

---

### Task 6: Add Windows Process Snapshot Collection

**Files:**
- Modify: `src/process/mod.rs`
- Test: `src/process/mod.rs`

- [ ] **Step 1: Add Windows parser tests behind `#[cfg(test)]`**

The parser tests should run on all platforms because they only parse fixture text.

```rust
#[test]
fn parses_powershell_process_csv_for_windows_snapshots() {
    let csv = "\"ProcessId\",\"CommandLine\",\"WorkingSetSize\"\r\n\"4321\",\"C:\\\\dst\\\\dontstarve_dedicated_server_nullrenderer_x64.exe -cluster ClusterWin -shard Master\",\"104857600\"\r\n";
    let snapshots = parse_windows_process_csv(csv);

    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].pid, Some(4321));
    assert_eq!(snapshots[0].resident_set_size, "102400");
    assert!(snapshots[0].matches_level("ClusterWin", "Master"));
}
```

- [ ] **Step 2: Run failing parser test**

Run:

```bash
cargo test process::tests::parses_powershell_process_csv_for_windows_snapshots
```

Expected: FAIL because the parser does not exist.

- [ ] **Step 3: Implement Windows provider using PowerShell CSV**

Replace the current `#[cfg(not(unix))] system_snapshots()` with a Windows-specific implementation and leave unsupported behavior for other non-Unix platforms.

```rust
#[cfg(windows)]
fn system_snapshots() -> io::Result<Vec<ProcessSnapshot>> {
    tracing::debug!(
        program = "powershell",
        "collecting Windows process snapshots"
    );
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_Process | Select-Object ProcessId,CommandLine,WorkingSetSize | ConvertTo-Csv -NoTypeInformation",
        ])
        .output()?;
    if !output.status.success() {
        tracing::warn!(
            status = ?output.status.code(),
            "PowerShell process query failed while collecting process snapshots"
        );
        return Err(io::Error::other("powershell process query failed"));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_windows_process_csv(&stdout))
}

#[cfg(all(not(unix), not(windows)))]
fn system_snapshots() -> io::Result<Vec<ProcessSnapshot>> {
    tracing::warn!("process snapshot collection is not implemented for this platform yet");
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "process snapshot collection is unsupported on this platform",
    ))
}
```

`parse_windows_process_csv` should parse quoted CSV for exactly three columns, set `cpu_usage` and `mem_usage` to empty strings when Windows cannot provide cheap equivalents, set `virtual_size` to empty, and convert `WorkingSetSize` bytes to KiB for `RSS`.

- [ ] **Step 4: Update support predicate**

```rust
pub fn system_snapshot_collection_supported() -> bool {
    cfg!(unix) || cfg!(windows)
}
```

- [ ] **Step 5: Run process tests**

Run:

```bash
cargo test process::
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/process/mod.rs
git commit -m "feat: collect Windows process snapshots"
```

---

### Task 7: Clean Migration Scaffolding And Stale Comments

**Files:**
- Modify: `src/handlers/compat.rs`
- Modify: `src/app.rs`
- Modify: comments in `src/handlers/cluster.rs`, `src/handlers/player.rs`, and tests whose names still say side effects are deferred.
- Test: `tests/compat_manifest_tests.rs`

- [ ] **Step 1: Remove dead stub router dependency from app assembly**

After every route is implemented and `route!(..., Stub)` count is zero, remove:

```rust
.merge(compat::stub_router())
```

from `src/app.rs`.

- [ ] **Step 2: Remove dead stub handler code**

In `src/handlers/compat.rs`, remove `not_migrated_handler`, `stub_router`, and imports used only by those functions. Keep `CompatibilityRouteStatus` and `COMPATIBILITY_ROUTE_MANIFEST` until the manifest tests are intentionally retired.

- [ ] **Step 3: Update comments and test names**

Rename tests/comments that now understate behavior:

- `cluster_routes_create_page_update_and_delete_without_starting_dst` becomes `cluster_routes_create_page_update_and_delete_with_existing_dst_install`.
- `Player list handlers backed by DST line-oriented list files` becomes `Player list and online-player handlers backed by DST files and console log queries`.
- The `cluster.rs` module comment should say create persists the row, installs DST when missing, and initializes cluster files.

- [ ] **Step 4: Run manifest tests**

Run:

```bash
cargo test --test compat_manifest_tests
```

Expected: PASS with zero remaining stubs.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/handlers/compat.rs src/handlers/cluster.rs src/handlers/player.rs tests/cluster_level_tests.rs
git commit -m "chore: remove migrated route stub scaffolding"
```

---

### Task 8: Full Verification And Review

**Files:**
- No feature files expected.
- Possible documentation update: this plan file and migration notes if any reviewer asks for traceability.

- [ ] **Step 1: Run full Rust formatting and linting**

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: PASS.

- [ ] **Step 2: Run full Rust tests**

```bash
cargo test
cargo test --locked
```

Expected: PASS.

- [ ] **Step 3: Build release binary**

```bash
cargo build --release --bin dst-admin-rust
```

Expected: PASS and produce `target/release/dst-admin-rust`.

- [ ] **Step 4: Run Go compatibility tests**

```bash
go test ./...
```

Expected: PASS. Go remains the compatibility oracle until the final repository cleanup decision.

- [ ] **Step 5: Run whitespace check**

```bash
git diff --check
```

Expected: no output.

- [ ] **Step 6: Request two read-only reviews**

Use one reviewer for spec compliance and one for code quality/security. Required review questions:

- Does `dst-admin-rust` now implement every non-security behavior identified in the Go/Rust parity gap list?
- Do cluster create, cluster list runtime fields, online players, system metrics, and Windows process snapshots have deterministic tests?
- Did any implementation weaken the accepted Rust security hardening differences?
- Are logs structured and useful without leaking command output or secrets?

- [ ] **Step 7: Address reviewer findings**

For each accepted finding, write a failing test first, implement the smallest correction, rerun the focused test, then rerun the relevant full verification subset.

- [ ] **Step 8: Final commit**

```bash
git status --short
git add docs/superpowers/plans/2026-06-17-rust-parity-completion.md
git commit -m "docs: plan remaining Rust parity work"
```

If implementation changes are already committed task-by-task, this final commit should contain only the plan document or be skipped when the plan was committed earlier.

---

## Execution Order

1. Task 1 first, because later route tests need injected process snapshots.
2. Task 2 next, because cluster create test fixtures feed cluster list and player tests.
3. Task 3 after Task 2, because runtime list enrichment depends on reliable cluster rows and files.
4. Task 4 after Task 1, because online players need process-provider status and command runner injection.
5. Task 5 can run in parallel with Task 3 or Task 4 after Task 1.
6. Task 6 can run in parallel after Task 1 because it only touches `src/process/mod.rs`.
7. Task 7 only after all route behavior is implemented.
8. Task 8 last.

## Acceptance Criteria

- `rg -c "route!\\(" src/handlers/compat.rs` remains `164` unless the manifest is deliberately retired.
- `rg "route!\\([^\\n]*Stub" src/handlers/compat.rs` returns no matches.
- Cluster create installs DST when `force_install_dir` is missing and rolls back the DB row on install or file initialization failure.
- Cluster list reports `master`, `caves`, `rowId`, `connected`, `maxConnections`, `mode`, `mods`, `season`, and `region` from the same sources as Go.
- Online player routes send the Go-compatible Lua query, read bounded recent logs, deduplicate by `kuId`, and return the Go `PlayerVO` JSON shape.
- System info route returns non-empty CPU/memory data on Linux while preserving Go's `panelCpuUsage` zero behavior.
- Windows process snapshot collection is implemented through a non-shell PowerShell argv invocation and parser tests.
- Full verification commands in Task 8 pass.
