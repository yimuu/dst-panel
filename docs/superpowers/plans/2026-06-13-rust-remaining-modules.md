# Rust Remaining Modules Migration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the remaining Rust compatibility stubs until `dst-admin-rust` has no Go route left behind a 501 stub.

**Architecture:** Continue the in-place Rust codebase and keep Go as the compatibility oracle until the final cleanup. Each module moves routes from `CompatibilityRouteStatus::Stub` to `Implemented` only after route-level parity tests cover the Go request contract, response envelope, side effects, logs, and safety boundaries. External network, SteamCMD, DST processes, websocket clients, timers, and filesystem archives must be behind fakeable Rust traits so tests do not require real external services.

**Tech Stack:** Rust 2024, axum, tokio, sqlx/sqlite, serde, tracing, reqwest, tokio process APIs, tower tests, tempfile, zip/tar helpers, WebSocket/SSE support.

---

## Current Baseline

As of `001f97f`, the compatibility manifest has:

- Total route entries: `164`
- Implemented: `89`
- Remaining stubs: `75`

`cargo test --test compat_manifest_tests` passes, so remaining work is explicitly represented by 501 stubs rather than hidden missing routes.

## Global Rules For Every Remaining Slice

- [ ] Start each route group with failing tests that prove the route is still a 501 stub or the Go-compatible behavior is absent.
- [ ] Preserve Go HTTP method, path, query/body names, JSON field names, status/envelope semantics, and odd legacy behavior unless there is a documented security exception.
- [ ] Keep all user input in SQL bind parameters; never concatenate raw query/body values into SQL or shell commands.
- [ ] Use safe path helpers for every file/archive/upload/static path; reject traversal, symlink escape, and unsafe archive entries.
- [ ] Use command traits/fakes for SteamCMD, DST server binaries, `screen`, archive tools, and map generation.
- [ ] Add comments for non-obvious Go compatibility quirks and structured `tracing` logs for operational events.
- [ ] Never log passwords, cookies, Steam API keys, cluster tokens, raw console commands, uploaded file contents, player chat, or IPs unless the Go API contract requires returning them.
- [ ] After each slice, run:

```bash
cargo fmt --all --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release --bin dst-admin-rust
cargo test --locked
go test ./...
git diff --check
```

- [ ] After each slice, start two read-only reviewers: one for spec compliance and one for code quality/security. Fix Critical and Important findings before closing the slice.

## Slice 1: Installation, First-Run Init, And Third-Party Read-Only Proxies

**Purpose:** Finish low-state setup/read-only endpoints before high-risk game operations.

**Routes:**

- `POST /api/init`
- `GET /api/install/steamcmd`
- `GET /api/dst/version`
- `POST /api/dst/home/server`
- `POST /api/dst/home/server/detail`
- `GET /api/dst/lobby/server/detail`
- `GET /api/dst/home/server2`
- `GET /api/dst/home/server/detail2`
- `GET /steam/dst/news`

**Likely files:**

- Create/modify: `src/handlers/init.rs`, `src/handlers/install.rs`, `src/handlers/third_party.rs`, `src/services/third_party.rs`, `src/app.rs`, `src/handlers/compat.rs`
- Tests: `tests/init_install_tests.rs`, `tests/third_party_tests.rs`, `tests/compat_manifest_tests.rs`

**Key tests:**

- [ ] `POST /api/init` writes the same first-run/config/password side effects as Go and is idempotent where Go is idempotent.
- [ ] SteamCMD install uses a fake command runner in tests and never shells through user-controlled strings.
- [ ] Third-party proxy routes use fake HTTP clients for success, timeout, malformed response, and upstream failure.
- [ ] Stub count decreases and all migrated routes stop returning 501.

## Slice 2: Mod Metadata, Workshop, UGC, Background Files, And `dst-static`

**Purpose:** Move mod/file workflows while enforcing path and upload safety.

**Routes:**

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
- All methods on `/api/dst-static/{*filepath}`

**Likely files:**

- Create/modify: `src/handlers/mods.rs`, `src/services/mods.rs`, `src/handlers/files.rs`, `src/handlers/dst_static.rs`, `src/fs_paths.rs`, `src/app.rs`, `src/handlers/compat.rs`
- Tests: `tests/mod_tests.rs`, `tests/file_upload_tests.rs`, `tests/dst_static_tests.rs`

**Key tests:**

- [ ] Mod ID validation rejects traversal and non-numeric/unsafe ids exactly where Go would fail or where Rust must harden.
- [ ] `modinfo.lua` and generated modinfo files round-trip fixtures without destructive formatting drift.
- [ ] UGC upload rejects archive traversal, symlink escapes, oversized unsafe files, and wrong content types where applicable.
- [ ] Background upload/get preserves Go response shape and does not serve outside the configured root.

## Slice 3: Game Lifecycle And Process Operations

**Purpose:** Migrate start/stop/update/preinstall/process commands using safe command adapters.

**Routes:**

- `GET /api/game/8level/start`
- `GET /api/game/8level/stop`
- `GET /api/game/8level/start/all`
- `GET /api/game/8level/stop/all`
- `GET /api/game/8level/udp/port`
- `GET /api/game/preinstall`
- `GET /api/game/update`
- `GET /api/game/operate/player`

**Likely files:**

- Create/modify: `src/game/lifecycle.rs`, `src/game/preinstall.rs`, `src/game/udp.rs`, `src/handlers/game_lifecycle.rs`, `src/process.rs`, `src/app.rs`, `src/handlers/compat.rs`
- Tests: `tests/game_lifecycle_tests.rs`, `tests/game_preinstall_tests.rs`

**Key tests:**

- [ ] Start/stop commands are argv-based, fakeable, and never execute shell strings in tests.
- [ ] Cluster/level validation runs before command construction.
- [ ] Start-all/stop-all preserve Go ordering and partial failure behavior.
- [ ] UDP scan route is bounded and does not scan uncontrolled host ranges.

## Slice 4: Backup, Archive, Restore, Snapshot Settings, And Share/Import

**Purpose:** Migrate the stateful archive workflows with transactional filesystem safety.

**Routes:**

- `GET /api/game/backup`
- `POST /api/game/backup`
- `DELETE /api/game/backup`
- `PUT /api/game/backup`
- `GET /api/game/backup/download`
- `POST /api/game/backup/upload`
- `GET /api/game/backup/restore`
- `GET /api/game/archive`
- `POST /api/game/backup/snapshot/setting`
- `GET /api/game/backup/snapshot/setting`
- `GET /api/game/backup/snapshot/list`
- `GET /api/share/keyCer`
- `GET /api/share/keyCer/reflush`
- `GET /api/share/keyCer/enable`
- `POST /api/share/cluster/import`
- `GET /share/cluster`

**Likely files:**

- Create/modify: `src/backup.rs`, `src/share.rs`, `src/handlers/backup.rs`, `src/handlers/share.rs`, `src/repositories/backup.rs`, `src/app.rs`, `src/handlers/compat.rs`
- Tests: `tests/backup_tests.rs`, `tests/share_tests.rs`

**Key tests:**

- [ ] Backup create/list/delete/update/download preserve Go filenames and envelope semantics.
- [ ] Restore and import validate archive contents before writing and roll back partial writes on failure.
- [ ] Snapshot settings persist in the same table/key semantics as Go.
- [ ] Share key/cert routes avoid logging or exposing secrets beyond the Go response contract.

## Slice 5: Announcements, Tasks, Auto-Check, Webhook, And Background Scheduling

**Purpose:** Migrate scheduler-like workflows after core file/process primitives are stable.

**Routes:**

- `GET /api/game/announce/setting`
- `POST /api/game/announce/setting`
- `GET /api/task`
- `POST /api/task`
- `DELETE /api/task`
- `GET /api/task/instruct`
- `GET /api/auto/check2`
- `POST /api/auto/check2`
- `POST /webhook`

**Likely files:**

- Create/modify: `src/scheduler.rs`, `src/handlers/tasks.rs`, `src/handlers/auto_check.rs`, `src/handlers/webhook.rs`, `src/repositories/tasks.rs`, `src/repositories/auto_check.rs`, `src/app.rs`, `src/handlers/compat.rs`
- Tests: `tests/task_tests.rs`, `tests/auto_check_tests.rs`, `tests/webhook_tests.rs`

**Key tests:**

- [ ] Task CRUD matches Go DB schema and soft-delete behavior.
- [ ] Instruct list response matches Go hard-coded/derived command list.
- [ ] Auto-check save/list preserves all fields and default values.
- [ ] Webhook authenticates/parses payloads without logging shared secrets.

## Slice 6: Map Generation And Session File Inspection

**Purpose:** Migrate map/session helpers with fakeable external generation.

**Routes:**

- `GET /api/dst/map/gen`
- `GET /api/dst/map/image`
- `GET /api/dst/map/has/walrusHut/plains`
- `GET /api/dst/map/session/file`
- `GET /api/dst/map/player/session/file`

**Likely files:**

- Create/modify: `src/map.rs`, `src/handlers/map.rs`, `src/app.rs`, `src/handlers/compat.rs`
- Tests: `tests/map_tests.rs`

**Key tests:**

- [ ] Map generation uses a fake process/service in tests and validates cluster/level paths before command construction.
- [ ] Image/session-file routes reject traversal and symlink escapes.
- [ ] Walrus hut/plains parser matches Go behavior on real fixture files.

## Slice 7: Streams And WebSocket

**Purpose:** Finish long-lived routes after their underlying data/process APIs exist.

**Routes:**

- `GET /ws`
- `GET /api/game/8level/status/stream`
- `GET /api/game/system/info/stream`
- `GET /api/game/log/stream`

**Likely files:**

- Create/modify: `src/streams.rs`, `src/ws.rs`, `src/handlers/streams.rs`, `src/handlers/ws.rs`, `src/app.rs`, `src/handlers/compat.rs`
- Tests: `tests/stream_tests.rs`, `tests/ws_tests.rs`

**Key tests:**

- [ ] Stream routes emit Go-compatible event cadence/shape for a fake status/log source.
- [ ] Disconnects cancel background work cleanly.
- [ ] WebSocket route validates session/auth behavior and does not leak panic details on malformed frames.

## Slice 8: Final Stub Removal, Release Parity, And Go Removal Decision

**Purpose:** Prove complete Rust parity and decide whether to remove Go or keep it as archived reference.

**Routes:**

- Every entry in `COMPATIBILITY_ROUTE_MANIFEST` must be `Implemented`.
- `stub_router` should be deleted or reduced to zero routes.

**Likely files:**

- Modify: `src/handlers/compat.rs`, `tests/compat_manifest_tests.rs`, release scripts, Docker files, install docs, README/install docs as needed.

**Key tests:**

- [ ] Manifest count shows `stub=0`.
- [ ] Unknown routes still return normal 404/405 and are not hidden by a catch-all.
- [ ] `dst-admin-rust` remains the target binary for release/docker/install paths.
- [ ] Full gate passes on a clean worktree.

## Expected Truncation / Slice Count

The remaining migration should take **8 implementation slices** from the current state:

1. Setup + third-party proxies
2. Mods + uploads + `dst-static`
3. Game lifecycle/process operations
4. Backup/archive/share
5. Scheduler/auto-check/webhook
6. Map/session helpers
7. Streams/WebSocket
8. Final parity and cleanup

If a slice exceeds the review budget, split it by route group before marking any partial group as complete.

## Execution Choice

Recommended execution mode: **Subagent-Driven**. Use one worker per slice for implementation, then run local full gate and two read-only reviewers before moving to the next slice.
