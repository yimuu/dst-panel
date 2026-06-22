# Rust Full Migration Follow-Up Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the Rust 2024 migration after the completed foundation slice until `dst-admin-rust` can replace the Go backend with no compatibility stubs and the Go backend can be removed.

**Architecture:** Keep the in-place Rust codebase as the target and keep Go as the compatibility oracle until the final cleanup slice. Every route moves from `CompatibilityRouteStatus::Stub` to `Implemented` only after parity tests cover the Go request contract, response envelope, file/database side effects, and operational logging. External SteamCMD, DST, and network behavior must be hidden behind Rust traits or command adapters so automated tests use fakes and never install or start a real DST server.

**Tech Stack:** Rust 2024, axum, tokio, sqlx/sqlite, serde/serde_json/serde_yaml, tracing, reqwest, chrono, uuid, zip, INI/Lua parsing crates, tokio process APIs, tower tests, tempfile.

---

## Completion Estimate

The foundation slice is complete at `aabd1f9`. A full migration should be planned as **8 additional implementation slices**. Counting the completed foundation, the full migration is expected to require **9 total slices**:

1. Completed: Rust foundation, auth, config, SQLite, static assets, low-risk KV/web-link routes, release binary rename.
2. Follow-up Slice 1: Compatibility harness hardening and shared primitives.
3. Follow-up Slice 2: Cluster, level, DST config, and player-list file workflows.
4. Follow-up Slice 3: Process control, game operations, installation, and system info.
5. Follow-up Slice 4: Logs, WebSocket/SSE streams, collectors, and statistics.
6. Follow-up Slice 5: Backup, archive, restore, and cluster share/import workflows.
7. Follow-up Slice 6: Mods, Steam Workshop, UGC uploads, and background files.
8. Follow-up Slice 7: Third-party proxies, Steam news, preinstall, and map generation.
9. Follow-up Slice 8: Scheduler, auto-check, webhook, final parity gate, and Go removal.

This split keeps each slice independently reviewable. If a slice is still too large during execution, split it by route group before writing code; do not merge a partial route as `Implemented`.

## Global Rules For Every Slice

- Do not mark a route as `Implemented` until its Rust behavior is parity-tested against the Go contract.
- Preserve existing paths, methods, query parameters, JSON field names, and response envelope semantics.
- Use fakes for SteamCMD, DST processes, filesystem fixtures, network clients, and cron timers.
- Log validation failures, command intent, file operations, background task lifecycle, and external request failures with `tracing`.
- Never log passwords, session cookies, Steam API keys, cluster tokens, raw console commands, or uploaded file contents.
- Keep Go code until Follow-up Slice 8 finishes and the final parity gate passes.
- Each slice ends with:

```bash
cargo fmt --all --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release --bin dst-admin-rust
cargo test --locked
go test ./...
git diff --check
```

## Shared Route Migration Pattern

Use this pattern for each route group:

- [ ] **Step 1: Add failing parity tests**

Create or extend a focused test file. The test must prove the current stub returns 501 before implementation or prove the new compatibility behavior is absent.

Example:

```rust
#[tokio::test]
async fn migrated_route_returns_go_compatible_envelope() {
    let (app, _dir) = test_router().await;
    let cookie = login(&app).await;

    let response = send(&app, Method::GET, "/api/example", None, Some(&cookie)).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await,
        json!({"code": 200, "msg": "success", "data": {}})
    );
}
```

- [ ] **Step 2: Run the focused test and confirm RED**

Run the smallest useful command, for example:

```bash
cargo test --test http_tests migrated_route_returns_go_compatible_envelope
```

Expected: FAIL because the route is still a 501 stub or because the handler does not exist.

- [ ] **Step 3: Implement the Rust handler and service**

Add the minimal module/service/repository behavior needed by the tests. Use explicit DTOs and comments for Go compatibility details.

- [ ] **Step 4: Move the route from stub to implemented**

Update all three places together:

- `src/handlers/compat.rs` route status
- `src/handlers/compat.rs` `stub_router` removal for that route
- `src/app.rs` real route registration

- [ ] **Step 5: Run focused tests and full verification**

Run the route test, manifest test, and global gate commands.

- [ ] **Step 6: Commit**

Use a narrow commit message:

```bash
git add <changed files>
git commit -m "feat: migrate <route group> to rust"
```

## Follow-Up Slice 1: Compatibility Harness Hardening And Shared Primitives

**Purpose:** Reduce migration risk before moving high-risk routes. This slice does not need to migrate many routes; it builds the reusable test and safety infrastructure needed by later slices.

**Files:**

- Modify: `tests/http_tests.rs`
- Modify: `tests/db_tests.rs`
- Modify: `tests/compat_manifest_tests.rs`
- Modify: `src/handlers/static_files.rs`
- Create: `src/validation.rs`
- Create: `src/fs_paths.rs`
- Create: `src/command.rs`
- Modify: `src/lib.rs`

**Scope:**

- Static path negative tests: traversal, encoded traversal, directory requests, and symlink escape.
- Table-driven additive migration tests for every table in `ADDITIVE_TABLE_SCHEMAS`.
- Central path and identifier validators for cluster names, level names, mod IDs, KU IDs, filenames, and backup archive names.
- Central command adapter trait used by later SteamCMD/DST operations.

**Acceptance Criteria:**

- Static asset requests cannot escape `dist`.
- Existing legacy SQLite tables missing any known column are repaired before indexes are created.
- Future service code can use `validation` and `command` instead of shell string concatenation.

**Commit:**

```bash
git commit -m "test: harden migration compatibility harness"
```

## Follow-Up Slice 2: Cluster, Level, DST Config, And Player Lists

**Purpose:** Migrate file-backed DST configuration and cluster CRUD without starting real game processes.

**Files:**

- Create: `src/dst/mod.rs`
- Create: `src/dst/cluster_ini.rs`
- Create: `src/dst/server_ini.rs`
- Create: `src/dst/lua_files.rs`
- Create: `src/dst/player_lists.rs`
- Create: `src/handlers/cluster.rs`
- Create: `src/handlers/level.rs`
- Create: `src/handlers/dst_config.rs`
- Create: `src/handlers/player.rs`
- Modify: `src/app.rs`
- Modify: `src/handlers/compat.rs`
- Modify: `src/handlers/mod.rs`
- Test: `tests/cluster_level_tests.rs`
- Test: `tests/dst_config_tests.rs`

**Routes To Migrate:**

- `GET /api/cluster`
- `POST /api/cluster`
- `PUT /api/cluster`
- `DELETE /api/cluster`
- `GET /api/cluster/level`
- `PUT /api/cluster/level`
- `POST /api/cluster/level`
- `DELETE /api/cluster/level`
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
- `GET /api/game/player`
- `GET /api/game/player/adminlist`
- `POST /api/game/player/adminlist`
- `DELETE /api/game/player/adminlist`
- `GET /api/game/player/blacklist`
- `POST /api/game/player/blacklist`
- `DELETE /api/game/player/blacklist`
- `GET /api/dst/config`
- `POST /api/dst/config`
- `GET /api/game/config`
- `POST /api/game/config`

**Acceptance Criteria:**

- `/api/cluster` list returns the Go-compatible paginated `Page` envelope and runtime `ClusterVO` shape.
- Cluster create generates UUID/server files and records DB changes transactionally, but external install/start operations remain behind fakes.
- Level config round-trips `cluster.ini`, `server.ini`, `leveldataoverride.lua`, and `modoverrides.lua` fixtures without destructive formatting drift.
- Player list APIs preserve line formats and reject traversal.

**Commit:**

```bash
git commit -m "feat: migrate cluster and dst config routes"
```

## Follow-Up Slice 3: Process Control, Game Operations, Install, And System Info

**Purpose:** Replace Go process control with safe Rust command execution and fakeable adapters.

**Files:**

- Create: `src/process/mod.rs`
- Create: `src/process/screen.rs`
- Create: `src/process/windows.rs`
- Create: `src/process/steamcmd.rs`
- Create: `src/game/mod.rs`
- Create: `src/handlers/game.rs`
- Modify: `src/app.rs`
- Modify: `src/handlers/compat.rs`
- Test: `tests/process_tests.rs`
- Test: `tests/game_process_tests.rs`

**Routes To Migrate:**

- `GET /api/install/steamcmd`
- `GET /api/game/preinstall`
- `GET /api/game/8level/status`
- `GET /api/game/8level/status/stream`
- `GET /api/game/8level/start`
- `GET /api/game/8level/stop`
- `GET /api/game/8level/start/all`
- `GET /api/game/8level/stop/all`
- `POST /api/game/8level/command`
- `GET /api/game/8level/udp/port`
- `GET /api/game/sent/broadcast`
- `GET /api/game/kick/player`
- `GET /api/game/kill/player`
- `GET /api/game/respawn/player`
- `GET /api/game/rollback`
- `GET /api/game/regenerateworld`
- `POST /api/game/master/console`
- `POST /api/game/caves/console`
- `GET /api/game/operate/player`
- `GET /api/game/update`
- `GET /api/game/system/info`
- `GET /api/game/system/info/stream`
- `GET /api/game/clean`
- `GET /api/game/clean/level`
- `GET /api/game/clean/level/all`

**Acceptance Criteria:**

- Command construction uses argument arrays or validated templates.
- Tests prove user inputs cannot inject shell commands.
- Status and stream routes work with fake process snapshots.
- Real command paths log sanitized intent, exit status, timeout, and kill behavior.

**Commit:**

```bash
git commit -m "feat: migrate dst process control"
```

## Follow-Up Slice 4: Logs, Collectors, WebSocket, SSE, And Statistics

**Purpose:** Migrate observability routes and background log parsing.

**Files:**

- Create: `src/logs/mod.rs`
- Create: `src/logs/tail.rs`
- Create: `src/logs/collector.rs`
- Create: `src/statistics.rs`
- Create: `src/handlers/logs.rs`
- Create: `src/handlers/statistics.rs`
- Modify: `src/app.rs`
- Modify: `src/handlers/compat.rs`
- Test: `tests/log_stream_tests.rs`
- Test: `tests/statistics_tests.rs`

**Routes To Migrate:**

- `GET /ws`
- `GET /api/game/log/stream`
- `GET /api/game/level/server/log`
- `GET /api/game/level/server/chat/log`
- `GET /api/game/level/server/download`
- `GET /api/game/dst-admin-go/log`
- `GET /api/game/dst-admin-go/log/download`
- `GET /api/player/log`
- `POST /api/player/log/delete`
- `GET /api/statistics/active/user`
- `GET /api/statistics/top/death`
- `GET /api/statistics/top/login`
- `GET /api/statistics/top/active`
- `GET /api/statistics/rate/role`
- `GET /api/statistics/regenerate`

**Acceptance Criteria:**

- Log snapshot/download APIs use safe path resolution.
- WebSocket/SSE tests use temporary files and deterministic tail data.
- Collector parses representative DST log fixtures into `spawns`, `connects`, `player_logs`, and `regenerates`.
- Statistics match Go query semantics for date windows, pagination, and empty data.

**Commit:**

```bash
git commit -m "feat: migrate logs and statistics"
```

## Follow-Up Slice 5: Backup, Archive, Restore, And Share

**Purpose:** Migrate backup lifecycle and cluster import/export without relying on real DST runtime.

**Files:**

- Create: `src/backup/mod.rs`
- Create: `src/backup/archive.rs`
- Create: `src/share.rs`
- Create: `src/handlers/backup.rs`
- Create: `src/handlers/share.rs`
- Modify: `src/app.rs`
- Modify: `src/handlers/compat.rs`
- Test: `tests/backup_tests.rs`
- Test: `tests/share_tests.rs`

**Routes To Migrate:**

- `GET /api/game/backup`
- `POST /api/game/backup`
- `DELETE /api/game/backup`
- `PUT /api/game/backup`
- `GET /api/game/backup/download`
- `POST /api/game/backup/upload`
- `GET /api/game/backup/restore`
- `POST /api/game/backup/snapshot/setting`
- `GET /api/game/backup/snapshot/setting`
- `GET /api/game/backup/snapshot/list`
- `GET /api/game/archive`
- `GET /api/share/keyCer`
- `GET /api/share/keyCer/reflush`
- `GET /api/share/keyCer/enable`
- `POST /api/share/cluster/import`
- `GET /share/cluster`

**Acceptance Criteria:**

- Backup names are validated and cannot traverse directories.
- Zip creation/extraction is tested with temporary DST save directories.
- Restore behavior updates files atomically enough to avoid partially restored worlds on failure.
- Share/import preserves existing cluster config formats and rejects unsafe archive entries.

**Commit:**

```bash
git commit -m "feat: migrate backup and sharing workflows"
```

## Follow-Up Slice 6: Mods, Steam Workshop, UGC Uploads, And Background Files

**Purpose:** Migrate mod search/install/config/update workflows and file upload endpoints.

**Files:**

- Create: `src/mods/mod.rs`
- Create: `src/mods/steam.rs`
- Create: `src/mods/lua_modinfo.rs`
- Create: `src/handlers/mods.rs`
- Create: `src/handlers/files.rs`
- Modify: `src/app.rs`
- Modify: `src/handlers/compat.rs`
- Test: `tests/mod_tests.rs`
- Test: `tests/file_upload_tests.rs`

**Routes To Migrate:**

- `GET /api/mod/search`
- `GET /api/mod/{modId}`
- `PUT /api/mod/{modId}`
- `GET /api/mod`
- `DELETE /api/mod/{modId}`
- `DELETE /api/mod/setup/workshop`
- `GET /api/mod/modinfo/{modId}`
- `POST /api/mod/modinfo`
- `POST /api/mod/modinfo/file`
- `PUT /api/mod/modinfo`
- `GET /api/mod/ugc/acf`
- `DELETE /api/mod/ugc`
- `POST /api/file/ugc/upload`
- `POST /api/file/background`
- `GET /api/file/background`

**Acceptance Criteria:**

- Steam Workshop HTTP calls are behind a trait and tested with fake JSON responses.
- SteamCMD mod download commands use the command adapter and validated `modId`.
- Lua `modinfo.lua` parsing is sandboxed or parser-based; tests cover representative Chinese/English mod metadata.
- Upload endpoints restrict filenames, size assumptions, and destination roots.

**Commit:**

```bash
git commit -m "feat: migrate mod and file workflows"
```

## Follow-Up Slice 7: Third-Party Proxies, Steam News, Preinstall, And Map Generation

**Purpose:** Migrate remaining network proxy and map/session helper routes.

**Files:**

- Create: `src/third_party/mod.rs`
- Create: `src/map/mod.rs`
- Create: `src/handlers/third_party.rs`
- Create: `src/handlers/map.rs`
- Modify: `src/app.rs`
- Modify: `src/handlers/compat.rs`
- Test: `tests/third_party_tests.rs`
- Test: `tests/map_tests.rs`

**Routes To Migrate:**

- `GET /api/dst/version`
- `POST /api/dst/home/server`
- `POST /api/dst/home/server/detail`
- `GET /api/dst/lobby/server/detail`
- `GET /api/dst/home/server2`
- `GET /api/dst/home/server/detail2`
- all methods on `/api/dst-static/{*filepath}`
- `GET /steam/dst/news`
- `GET /api/dst/map/gen`
- `GET /api/dst/map/image`
- `GET /api/dst/map/has/walrusHut/plains`
- `GET /api/dst/map/session/file`
- `GET /api/dst/map/player/session/file`

**Acceptance Criteria:**

- Network clients are fakeable and never hit real endpoints in normal tests.
- Proxy routes preserve status/body behavior where the frontend depends on it.
- Static proxy path validation rejects external URL/path abuse.
- Map/session helpers work from fixture save files.

**Commit:**

```bash
git commit -m "feat: migrate third party and map routes"
```

## Follow-Up Slice 8: Scheduler, Auto-Check, Webhook, Final Parity Gate, And Go Removal

**Purpose:** Finish background jobs and remove the Go backend only after every route has Rust parity.

**Files:**

- Create: `src/scheduler/mod.rs`
- Create: `src/auto_check/mod.rs`
- Create: `src/handlers/tasks.rs`
- Create: `src/handlers/auto_check.rs`
- Create: `src/handlers/webhook.rs`
- Modify: `src/app.rs`
- Modify: `src/handlers/compat.rs`
- Modify: `README.md`
- Modify: `README-EN.md`
- Modify: `docs/install.md`
- Delete after final gate: `main.go`, `go.mod`, `go.sum`, `api/`, `bootstrap/`, `collect/`, `config/` Go package files, `constant/`, `middleware/`, `model/`, `router/`, `schedule/`, `service/`, `utils/`, `vo/`
- Test: `tests/scheduler_tests.rs`
- Test: `tests/auto_check_tests.rs`
- Test: `tests/final_parity_tests.rs`

**Routes To Migrate:**

- `GET /api/task`
- `POST /api/task`
- `DELETE /api/task`
- `GET /api/task/instruct`
- `GET /api/auto/check2`
- `POST /api/auto/check2`
- `POST /webhook`

**Final Parity Requirements:**

- `COMPATIBILITY_ROUTE_MANIFEST` contains no `CompatibilityRouteStatus::Stub`.
- No route in `stub_router` remains except if the function is deleted with tests proving no stubs exist.
- `cargo test --locked` passes from a clean checkout.
- `cargo build --release --bin dst-admin-rust --locked` passes.
- Docker image starts `dst-admin-rust` and serves the frontend shell plus static assets.
- Manual smoke test against a disposable DST fixture confirms login, config read/write, cluster create, level create, backup create/restore, mod metadata, process fake/real command dry run, and task scheduling.
- Go backend files are removed only after all automated gates pass.

**Commit Sequence:**

Use at least two commits:

```bash
git commit -m "feat: migrate scheduler auto check and webhook"
git commit -m "chore: remove go backend after rust parity"
```

## Residual Risks To Track

- Real DST and SteamCMD behavior cannot be fully proven in unit tests. Keep a separate manual release checklist with a disposable server.
- Windows process support needs explicit review because the Go implementation has separate Windows services.
- Lua parsing is the highest-risk parser area. Prefer a constrained parser or sandboxed embedded Lua, and never execute untrusted mod code with unrestricted globals.
- Full route parity may reveal frontend assumptions that are not obvious from Go handlers. Preserve the explicit manifest and add a browser smoke test before final Go removal.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-11-rust-migration-followup.md`. Two execution options:

1. **Subagent-Driven (recommended)** - Dispatch a fresh subagent per slice, review between slices, and only move routes from `Stub` to `Implemented` after tests pass.
2. **Inline Execution** - Execute slices in this session using `superpowers:executing-plans`, with a review checkpoint after each slice.
