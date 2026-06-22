# Rust Module Structure Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure the Rust 2024 `dst-admin-rust` backend into a clearer feature-first module layout without changing any HTTP contract, persistence behavior, command behavior, or accepted security hardening differences.

**Architecture:** Refactor in two phases: first create stable `infra/`, `web/`, and `domain/` module roots with compatibility re-exports so existing imports keep compiling; then move feature code next to the domain it belongs to and remove obsolete top-level shims only after the full test suite passes. Each task is mechanical, independently testable, and should be committed before moving to the next task.

**Tech Stack:** Rust 2024, Axum 0.8, Tokio, SQLx SQLite, Reqwest-backed `HttpClient`, `tracing`, existing integration tests under `tests/`, `cargo fmt`, `cargo clippy`, `cargo test`.

---

## Scope

This is a no-behavior-change refactor.

Allowed:

- Move files into clearer modules.
- Rename internal module paths.
- Add `mod.rs` files, module-level documentation, and `pub use` compatibility shims.
- Split oversized files when the split is mechanical and covered by existing tests.
- Improve internal visibility from `pub` to `pub(crate)` only when tests and callers prove it is not public API.

Not allowed:

- Change route paths, methods, auth behavior, request/response JSON shape, status codes, legacy typo fields, or Go-compatible messages.
- Change database schema, migration SQL, soft-delete behavior, or repository semantics.
- Change command argv construction, process matching, safe path behavior, or symlink/traversal hardening.
- Combine this refactor with performance changes, new features, or dependency upgrades.

## Target Structure

Final target:

```text
src/
  lib.rs
  main.rs

  web/
    mod.rs
    app.rs
    error.rs
    response.rs
    handlers/
      mod.rs
      announcement.rs
      auth.rs
      auto_check.rs
      backup.rs
      cluster.rs
      compat.rs
      dst_config.rs
      dst_static.rs
      files.rs
      game.rs
      init.rs
      install.rs
      kv.rs
      level.rs
      logs.rs
      map.rs
      mods.rs
      player.rs
      player_log.rs
      share.rs
      static_files.rs
      statistics.rs
      steam_news.rs
      streams.rs
      tasks.rs
      third_party.rs
      web_link.rs
      webhook.rs
      ws.rs

  infra/
    mod.rs
    command.rs
    config.rs
    db.rs
    fs_paths.rs
    http_client.rs
    logging.rs
    process/
      mod.rs

  domain/
    mod.rs
    auth/
      mod.rs
      service.rs
    cluster/
      mod.rs
      install.rs
      model.rs
      repository.rs
      runtime.rs
    game/
      mod.rs
      console.rs
      lifecycle.rs
      player_query.rs
      preinstall.rs
      status.rs
      udp.rs
    backup/
      mod.rs
      model.rs
      repository.rs
      service.rs
    map/
      mod.rs
      service.rs
    mods/
      mod.rs
      model.rs
      repository.rs
    scheduler/
      mod.rs
      model.rs
      repository.rs
    statistics/
      mod.rs
      repository.rs
    admin/
      mod.rs
      model.rs
      repositories.rs

  dst/
    mod.rs
    cluster_ini.rs
    lua_files.rs
    player_lists.rs
    server_ini.rs

  logs/
    mod.rs
  validation.rs
```

Temporary compatibility shims are allowed while moving:

```rust
pub mod infra;
pub mod web;
pub mod domain;

pub use domain::{auth, backup, game};
pub use domain::map as dst_map;
pub use infra::{command, config, db, fs_paths, http_client, logging, process};
pub use web::{app, error, handlers, response};
```

The final task decides whether to keep these root-level re-exports for integration-test ergonomics or remove them and update all callers to the final paths.

## Verification Gate For Every Task

Run at least the focused tests listed in the task. Before each commit, run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
```

If a task moves public module paths used by many tests, also run:

```bash
cargo test
```

Final verification after the last task:

```bash
cargo fmt --all --check
git diff --check
cargo clippy --all-targets --all-features -- -D warnings
go test ./...
cargo test
cargo test --locked
cargo build --release --bin dst-admin-rust
```

---

### Task 1: Baseline And Architecture Guardrails

**Files:**
- Create: `docs/architecture/rust-module-layout.md`
- Modify: `docs/superpowers/plans/2026-06-18-rust-module-structure-refactor.md` only for checkbox progress

- [x] **Step 1: Record the current public module surface**

Run:

```bash
find src -maxdepth 2 -type f | sort
rg -n "^pub mod|^pub use" src/lib.rs src/**/*.rs
wc -l src/models.rs src/backup.rs src/dst_map.rs src/auth.rs src/game/*.rs src/handlers/*.rs src/repositories/*.rs
```

Expected:

- Current top-level modules include `app`, `auth`, `backup`, `command`, `config`, `db`, `dst_map`, `error`, `fs_paths`, `game`, `handlers`, `http_client`, `logging`, `models`, `process`, `repositories`, `response`.
- Large files to split later include `src/handlers/mods.rs`, `src/backup.rs`, `src/dst_map.rs`, `src/models.rs`, and `src/game/lifecycle.rs`.

- [x] **Step 2: Create the architecture note**

Create `docs/architecture/rust-module-layout.md` with:

```markdown
# Rust Module Layout

The Rust backend uses feature-first modules with three root groups:

- `web`: HTTP application assembly, route handlers, response envelopes, and web errors.
- `infra`: external boundaries such as commands, HTTP clients, process snapshots, database connection, config loading, logging, and safe filesystem primitives.
- `domain`: DST admin business domains such as cluster, game lifecycle, backup, mods, map, auth, scheduler, and statistics.

`dst` remains a root module because it models DST file formats and path conventions rather than one panel feature.

During the refactor, `src/lib.rs` may re-export moved modules at their old root paths so existing integration tests and internal imports continue to compile. These shims are temporary unless explicitly kept in the final cleanup task.

No route behavior, JSON response shape, database schema, command argv construction, or safety hardening may change during this refactor.
```

- [x] **Step 3: Run baseline verification**

Run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Expected: all pass before any file movement.

- [x] **Step 4: Commit**

```bash
git add docs/architecture/rust-module-layout.md docs/superpowers/plans/2026-06-18-rust-module-structure-refactor.md
git commit -m "docs: define rust module layout refactor"
```

---

### Task 2: Move Infrastructure Boundaries Under `infra`

**Files:**
- Create: `src/infra/mod.rs`
- Move: `src/command.rs` -> `src/infra/command.rs`
- Move: `src/config.rs` -> `src/infra/config.rs`
- Move: `src/db.rs` -> `src/infra/db.rs`
- Move: `src/fs_paths.rs` -> `src/infra/fs_paths.rs`
- Move: `src/http_client.rs` -> `src/infra/http_client.rs`
- Move: `src/logging.rs` -> `src/infra/logging.rs`
- Move: `src/process/mod.rs` -> `src/infra/process/mod.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Move files mechanically**

Run:

```bash
mkdir -p src/infra
git mv src/command.rs src/infra/command.rs
git mv src/config.rs src/infra/config.rs
git mv src/db.rs src/infra/db.rs
git mv src/fs_paths.rs src/infra/fs_paths.rs
git mv src/http_client.rs src/infra/http_client.rs
git mv src/logging.rs src/infra/logging.rs
git mv src/process src/infra/process
```

- [x] **Step 2: Add `src/infra/mod.rs`**

```rust
//! Infrastructure boundaries for the Rust backend.
//!
//! These modules isolate external systems and process-wide services from
//! domain logic: command execution, HTTP, SQLite, configuration, logging,
//! process snapshots, and safe filesystem primitives.

pub mod command;
pub mod config;
pub mod db;
pub mod fs_paths;
pub mod http_client;
pub mod logging;
pub mod process;
```

- [x] **Step 3: Replace the old top-level declarations in `src/lib.rs`**

Change `src/lib.rs` from:

```rust
pub mod command;
pub mod config;
pub mod db;
pub mod fs_paths;
pub mod http_client;
pub mod logging;
pub mod process;
```

to:

```rust
pub mod infra;

pub use infra::{command, config, db, fs_paths, http_client, logging, process};
```

Keep every other existing `pub mod` line unchanged for this task.

- [x] **Step 4: Compile check the re-export shim**

Run:

```bash
cargo check
cargo test --test shared_primitives_tests
cargo test --test config_tests
cargo test --test db_tests
cargo test --test game_process_tests
```

Expected: all pass with old import paths such as `dst_admin_rust::command` still working.

- [x] **Step 5: Full task verification**

Run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
cargo test
```

- [x] **Step 6: Commit**

```bash
git add src/lib.rs src/infra
git commit -m "refactor: move infrastructure modules under infra"
```

---

### Task 3: Move Web Assembly, Errors, Responses, And Handlers Under `web`

**Files:**
- Create: `src/web/mod.rs`
- Move: `src/app.rs` -> `src/web/app.rs`
- Move: `src/error.rs` -> `src/web/error.rs`
- Move: `src/response.rs` -> `src/web/response.rs`
- Move: `src/handlers/` -> `src/web/handlers/`
- Modify: `src/lib.rs`

- [x] **Step 1: Move files mechanically**

Run:

```bash
mkdir -p src/web
git mv src/app.rs src/web/app.rs
git mv src/error.rs src/web/error.rs
git mv src/response.rs src/web/response.rs
git mv src/handlers src/web/handlers
```

- [x] **Step 2: Add `src/web/mod.rs`**

```rust
//! HTTP-facing application layer.
//!
//! `web` owns Axum app assembly, handlers, legacy response envelopes, and
//! web-facing errors. Domain and infrastructure modules must not depend on
//! concrete Axum router assembly.

pub mod app;
pub mod error;
pub mod handlers;
pub mod response;
```

- [x] **Step 3: Replace old web declarations in `src/lib.rs`**

Change:

```rust
pub mod app;
pub mod error;
pub mod handlers;
pub mod response;
```

to:

```rust
pub mod web;

pub use web::{app, error, handlers, response};
```

- [x] **Step 4: Keep old imports compiling**

Run:

```bash
cargo check
```

Expected: existing `crate::handlers`, `crate::error`, `crate::response`, and `dst_admin_rust::app` paths still compile through root re-exports.

- [x] **Step 5: Focused web verification**

Run:

```bash
cargo test --test http_tests
cargo test --test auth_tests
cargo test --test compat_manifest_tests
```

Expected: all pass; route registration and response envelopes are unchanged.

- [x] **Step 6: Full task verification**

Run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
cargo test
```

- [x] **Step 7: Commit**

```bash
git add src/lib.rs src/web
git commit -m "refactor: move web modules under web"
```

---

### Task 4: Move Core Domain Services Under `domain`

**Files:**
- Create: `src/domain/mod.rs`
- Create: `src/domain/auth/mod.rs`
- Create: `src/domain/backup/mod.rs`
- Create: `src/domain/map/mod.rs`
- Move: `src/auth.rs` -> `src/domain/auth/service.rs`
- Move: `src/backup.rs` -> `src/domain/backup/service.rs`
- Move: `src/dst_map.rs` -> `src/domain/map/service.rs`
- Move: `src/game/` -> `src/domain/game/`
- Modify: `src/lib.rs`

- [x] **Step 1: Move files mechanically**

Run:

```bash
mkdir -p src/domain/auth src/domain/backup src/domain/map
git mv src/auth.rs src/domain/auth/service.rs
git mv src/backup.rs src/domain/backup/service.rs
git mv src/dst_map.rs src/domain/map/service.rs
git mv src/game src/domain/game
```

- [x] **Step 2: Add `src/domain/mod.rs`**

```rust
//! Feature domains for the DST admin backend.
//!
//! Domain modules contain business behavior and DTOs that are independent from
//! Axum route assembly. Handlers call into these modules; infrastructure is
//! passed in through fakeable traits.

pub mod auth;
pub mod backup;
pub mod game;
pub mod map;
```

- [x] **Step 3: Add auth, backup, and map module facades**

`src/domain/auth/mod.rs`:

```rust
//! Authentication domain services and session helpers.

mod service;

pub use service::*;
```

`src/domain/backup/mod.rs`:

```rust
//! Backup, archive, restore, and snapshot domain services.

mod service;

pub(crate) use service::*;
```

`src/domain/map/mod.rs`:

```rust
//! Map-generation and session-file domain services.

mod service;

pub use service::*;
```

- [x] **Step 4: Replace old domain declarations in `src/lib.rs`**

Change:

```rust
pub mod auth;
pub mod backup;
pub mod dst_map;
pub mod game;
```

to:

```rust
pub mod domain;

pub use domain::{auth, backup, game};
pub use domain::map as dst_map;
```

- [x] **Step 5: Compile check old paths**

Run:

```bash
cargo check
```

Expected: old paths such as `crate::auth::SessionStore`, `crate::backup::*`, `crate::dst_map::*`, and `crate::game::*` still compile through re-exports.

- [x] **Step 6: Focused verification**

Run:

```bash
cargo test --test auth_tests
cargo test --test backup_share_tests
cargo test --test map_tests
cargo test --test game_lifecycle_tests
cargo test --test game_process_tests
cargo test --test player_query_tests
```

- [x] **Step 7: Full task verification**

Run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
cargo test
```

- [x] **Step 8: Commit**

```bash
git add src/lib.rs src/domain
git commit -m "refactor: move core domain services under domain"
```

---

### Task 5: Extract Cluster-Specific Runtime And Install Code From Game Domain

**Files:**
- Create: `src/domain/cluster/mod.rs`
- Move: `src/domain/game/cluster_runtime.rs` -> `src/domain/cluster/runtime.rs`
- Move: `src/domain/game/install.rs` -> `src/domain/cluster/install.rs`
- Modify: `src/domain/mod.rs`
- Modify: `src/domain/game/mod.rs`
- Modify: `src/web/handlers/cluster.rs`

- [x] **Step 1: Move files mechanically**

Run:

```bash
mkdir -p src/domain/cluster
git mv src/domain/game/cluster_runtime.rs src/domain/cluster/runtime.rs
git mv src/domain/game/install.rs src/domain/cluster/install.rs
```

- [x] **Step 2: Add cluster domain facade**

Create `src/domain/cluster/mod.rs`:

```rust
//! Cluster lifecycle support that is shared by cluster HTTP routes.
//!
//! This module owns persisted-cluster runtime enrichment and DST dedicated
//! server installation helpers. It stays separate from `domain::game`, which is
//! reserved for active shard/gameplay operations.

pub(crate) mod install;
pub(crate) mod runtime;
```

- [x] **Step 3: Register cluster domain**

Add this line to `src/domain/mod.rs`:

```rust
pub mod cluster;
```

- [x] **Step 4: Remove stale game submodule declarations**

In `src/domain/game/mod.rs`, remove:

```rust
pub(crate) mod cluster_runtime;
pub(crate) mod install;
```

Do not change other `game` module declarations.

- [x] **Step 5: Update cluster handler imports**

In `src/web/handlers/cluster.rs`, replace references:

```rust
game::cluster_runtime::ClusterRuntimeInfo
game::cluster_runtime::collect_lobby_rows_for_clusters
game::cluster_runtime::collect_for_cluster
game::install::install_dedicated_server_if_missing
```

with:

```rust
crate::domain::cluster::runtime::ClusterRuntimeInfo
crate::domain::cluster::runtime::collect_lobby_rows_for_clusters
crate::domain::cluster::runtime::collect_for_cluster
crate::domain::cluster::install::install_dedicated_server_if_missing
```

Importing `crate::domain::cluster::{install as cluster_install, runtime as cluster_runtime}` and
calling through those aliases is acceptable. Also remove any now-unused `game` import from that handler.

- [x] **Step 6: Verify cluster behavior**

Run:

```bash
cargo test --test cluster_level_tests
cargo test --test compat_manifest_tests
```

Expected: cluster CRUD, install-on-create, runtime fields, and large-page lobby enrichment still pass.

- [x] **Step 7: Full task verification**

Run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
cargo test
```

- [x] **Step 8: Commit**

```bash
git add src/domain src/web/handlers/cluster.rs
git commit -m "refactor: separate cluster domain from game domain"
```

---

### Task 6: Move Persistence Modules To Domain Or Admin Groups

**Files:**
- Create/modify: `src/domain/cluster/repository.rs`
- Create/modify: `src/domain/backup/repository.rs`
- Create/modify: `src/domain/mods/repository.rs`
- Create/modify: `src/domain/scheduler/repository.rs`
- Create/modify: `src/domain/statistics/repository.rs`
- Create/modify: `src/domain/admin/repositories.rs`
- Move: repository files from `src/repositories/`
- Modify: `src/domain/mod.rs`
- Modify: `src/repositories/mod.rs` or remove it in final cleanup
- Modify: handlers/services that use repository paths

- [x] **Step 1: Create domain module roots for remaining groups**

Create `src/domain/mods/mod.rs`:

```rust
//! Mod metadata, local mod files, Steam workshop cache, and UGC domain.

pub mod repository;
```

Create `src/domain/scheduler/mod.rs`:

```rust
//! Scheduled task, auto-check, and announcement persistence domain.

pub mod repository;
```

Create `src/domain/statistics/mod.rs`:

```rust
//! Player statistics and log aggregation domain.

pub mod repository;
```

Create `src/domain/admin/mod.rs`:

```rust
//! Small admin panel data domains such as KV and web links.

pub mod repositories;
```

Add to `src/domain/mod.rs`:

```rust
pub mod admin;
pub mod mods;
pub mod scheduler;
pub mod statistics;
```

- [x] **Step 2: Move repository files**

Run:

```bash
git mv src/repositories/cluster.rs src/domain/cluster/repository.rs
git mv src/repositories/backup.rs src/domain/backup/repository.rs
git mv src/repositories/mod_info.rs src/domain/mods/repository.rs
git mv src/repositories/statistics.rs src/domain/statistics/repository.rs
```

For scheduler/admin/statistics repositories, move multiple existing repository files into grouped
module directories only if the move is mechanical. The final targets are:

```bash
git mv src/repositories/announcement.rs src/domain/scheduler/repository/announcement.rs
git mv src/repositories/auto_check.rs src/domain/scheduler/repository/auto_check.rs
git mv src/repositories/tasks.rs src/domain/scheduler/repository/tasks.rs
git mv src/repositories/kv.rs src/domain/admin/repositories/kv.rs
git mv src/repositories/web_link.rs src/domain/admin/repositories/web_link.rs
git mv src/repositories/player_log.rs src/domain/statistics/repository/player_log.rs
```

Then make `src/domain/scheduler/repository.rs`:

```rust
//! Scheduler persistence facade.

pub mod announcement;
pub mod auto_check;
pub mod tasks;
```

Make `src/domain/admin/repositories.rs`:

```rust
//! Admin persistence facade.

pub mod kv;
pub mod web_link;
```

Make `src/domain/statistics/repository.rs` expose the moved player-log submodule:

```rust
pub mod player_log;
```

The migrated contents of `src/repositories/statistics.rs` stay in
`src/domain/statistics/repository.rs` so Rust does not need both
`repository.rs` and `repository/mod.rs`.

- [x] **Step 3: Keep old repository paths temporarily**

Replace `src/repositories/mod.rs` with compatibility re-exports:

```rust
//! Compatibility re-exports for repository modules during the domain refactor.

pub use crate::domain::admin::repositories::{kv, web_link};
pub use crate::domain::backup::repository as backup;
pub use crate::domain::cluster::repository as cluster;
pub use crate::domain::mods::repository as mod_info;
pub use crate::domain::scheduler::repository::{announcement, auto_check, tasks};
pub use crate::domain::statistics::repository as statistics;
pub use crate::domain::statistics::repository::player_log;
```

Keep `pub mod repositories;` in `src/lib.rs` for this task.

- [x] **Step 4: Compile and fix module path declarations**

Run:

```bash
cargo check
```

If Rust reports a missing module file for a repository facade, fix the `mod.rs`/directory shape without changing repository code.

- [x] **Step 5: Focused repository verification**

Run:

```bash
cargo test --test db_tests
cargo test --test cluster_level_tests
cargo test --test backup_share_tests
cargo test --test mod_file_static_tests
cargo test --test scheduler_auto_check_tests
cargo test --test statistics_tests
```

- [x] **Step 6: Full task verification**

Run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
cargo test
```

- [x] **Step 7: Commit**

```bash
git add src/domain src/repositories
git commit -m "refactor: move repositories into domain modules"
```

---

### Task 7: Split `models.rs` Into Domain Model Files

**Files:**
- Create: `src/domain/admin/model.rs`
- Create: `src/domain/backup/model.rs`
- Create: `src/domain/cluster/model.rs`
- Create: `src/domain/mods/model.rs`
- Create: `src/domain/scheduler/model.rs`
- Create: `src/domain/statistics/model.rs`
- Modify: `src/models.rs`
- Modify: repository and handler imports only if they do not compile through the compatibility facade

- [x] **Step 1: Move model groups by exact struct names**

Move these structs from `src/models.rs` to `src/domain/admin/model.rs`:

```rust
KvRecord
NewWebLink
WebLinkRecord
```

Move these structs to `src/domain/cluster/model.rs`:

```rust
NewCluster
ClusterRecord
```

Move these structs to `src/domain/statistics/model.rs`:

```rust
PlayerLogRecord
RegenerateRecord
```

Move these structs to `src/domain/backup/model.rs`:

```rust
BackupSnapshotRecord
SaveBackupSnapshot
```

Move these structs to `src/domain/scheduler/model.rs`:

```rust
AnnounceRecord
SaveAnnounce
JobTaskRecord
SaveJobTask
AutoCheckRecord
SaveAutoCheck
```

Move these structs to `src/domain/mods/model.rs`:

```rust
ModInfoRecord
ModInfoInput
```

Also move any helper functions used only by a moved group into that same model file. Keep shared timestamp serialization helpers in `src/models.rs` until the compatibility facade is removed.

- [x] **Step 2: Add model exports to each domain module**

For each domain `mod.rs`, add:

```rust
pub mod model;
```

where the domain has a `model.rs`.

- [x] **Step 3: Turn `src/models.rs` into a compatibility facade**

Replace `src/models.rs` with:

```rust
//! Compatibility re-exports for database row and input models.
//!
//! New code should import models from their owning domain module.

pub use crate::domain::admin::model::{KvRecord, NewWebLink, WebLinkRecord};
pub use crate::domain::backup::model::{BackupSnapshotRecord, SaveBackupSnapshot};
pub use crate::domain::cluster::model::{ClusterRecord, NewCluster};
pub use crate::domain::mods::model::{ModInfoInput, ModInfoRecord};
pub use crate::domain::scheduler::model::{
    AnnounceRecord, AutoCheckRecord, JobTaskRecord, SaveAnnounce, SaveAutoCheck, SaveJobTask,
};
pub use crate::domain::statistics::model::{PlayerLogRecord, RegenerateRecord};
```

If any moved model needs the existing `serialize_gorm_time` helper, move that helper into the domain model file that uses it.

- [x] **Step 4: Compile and fix imports**

Run:

```bash
cargo check
```

Expected: existing imports from `crate::models::*` continue to compile.

- [x] **Step 5: Focused model verification**

Run:

```bash
cargo test --test db_tests
cargo test --test cluster_level_tests
cargo test --test backup_share_tests
cargo test --test mod_file_static_tests
cargo test --test scheduler_auto_check_tests
cargo test --test statistics_tests
```

- [x] **Step 6: Full task verification**

Run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
cargo test
```

- [x] **Step 7: Commit**

```bash
git add src/models.rs src/domain
git commit -m "refactor: split models by domain"
```

---

### Task 8: Split Oversized Feature Files After They Are In Their Domains

**Files:**
- Modify/split: `src/domain/backup/service.rs`
- Modify/split: `src/web/handlers/mods.rs`
- Modify/split: `src/domain/map/service.rs`
- Modify/split: `src/domain/game/lifecycle.rs`

- [x] **Step 1: Split backup service by responsibility**

Create these files:

```text
src/domain/backup/archive.rs
src/domain/backup/restore.rs
src/domain/backup/share.rs
src/domain/backup/snapshot.rs
```

Move functions from `service.rs` by behavior:

- archive creation/list/download/delete into `archive.rs`
- restore upload/zip validation/swap logic into `restore.rs`
- share-key and share-cluster behavior into `share.rs`
- snapshot setting/list behavior into `snapshot.rs`

Keep `src/domain/backup/service.rs` as a facade over the service modules. The current
service file only contains archive and restore behavior; `share.rs` and
`snapshot.rs` are reserved module boundaries until the handler/repository logic
moves down into the backup domain.

```rust
//! Backup domain facade.

pub(crate) use super::archive::*;
pub(crate) use super::restore::*;
```

Run:

```bash
cargo test --test backup_share_tests
```

- [x] **Step 2: Split mods handler by route family**

Create:

```text
src/web/handlers/mods/
  mod.rs
  db.rs
  local.rs
  manual.rs
  search.rs
  steam.rs
  ugc.rs
```

Move route handlers from `src/web/handlers/mods.rs` by route family, then delete the old file. The new `mod.rs` must re-export all handlers that `src/web/app.rs` mounts. Because the handlers are crate-private, keep the re-exports crate-private:

```rust
//! Mod route handlers grouped by route family.

mod db;
mod local;
mod manual;
mod search;
mod steam;
mod ugc;

pub(crate) use db::*;
pub(crate) use local::*;
pub(crate) use manual::*;
pub(crate) use search::*;
pub(crate) use steam::*;
pub(crate) use ugc::*;
```

Run:

```bash
cargo test --test mod_file_static_tests
cargo test --test http_tests cluster_collection_routes_are_migrated_from_compatibility_stubs
```

- [x] **Step 3: Split map service by parser/generator/session responsibilities**

Create:

```text
src/domain/map/generator.rs
src/domain/map/session.rs
src/domain/map/player_session.rs
```

Keep `src/domain/map/service.rs` as:

```rust
//! Map domain facade.

pub use super::generator::*;
pub use super::player_session::*;
pub use super::session::*;
```

Run:

```bash
cargo test --test map_tests
```

- [x] **Step 4: Split game lifecycle command groups**

Create:

```text
src/domain/game/start_stop.rs
src/domain/game/update.rs
src/domain/game/clean.rs
```

Move functions without changing signatures:

- start/stop/start-all/stop-all into `start_stop.rs`
- update-game/update-bin/preinstall coordination into `update.rs`
- clean-world/clean-level/clean-all-level remain in `web::handlers::game`; `clean.rs`
  is a reserved boundary because those functions were not in `domain/game/lifecycle.rs`
  in this refactor slice.

Keep `src/domain/game/lifecycle.rs` as the lifecycle facade plus shared command helpers:

```rust
//! Game lifecycle facade.

#[path = "clean.rs"]
mod clean;
#[path = "start_stop.rs"]
mod start_stop;
#[path = "update.rs"]
mod update;

pub(crate) use start_stop::*;
pub(crate) use update::*;
```

Run:

```bash
cargo test --test game_lifecycle_tests
cargo test --test game_clean_tests
```

- [x] **Step 5: Full task verification**

Run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
cargo test
```

- [x] **Step 6: Commit**

```bash
git add src/domain src/web
git commit -m "refactor: split oversized feature modules"
```

---

### Task 9: Rewrite Imports To Final Paths And Remove Temporary Shims

**Files:**
- Modify: `src/lib.rs`
- Modify: all files importing root compatibility modules
- Delete or empty: `src/repositories/mod.rs` if no longer used as public facade
- Modify: tests only where they import root compatibility modules directly

- [x] **Step 1: Find all root shim imports**

Run:

```bash
rg -n "crate::(app|auth|backup|command|config|db|dst_map|error|fs_paths|game|handlers|http_client|logging|models|process|repositories|response)::" src tests
rg -n "dst_admin_rust::(app|auth|backup|command|config|db|dst_map|error|fs_paths|game|handlers|http_client|logging|models|process|repositories|response)::" tests
```

- [x] **Step 2: Rewrite imports to final paths**

Use these exact mappings:

```text
crate::app::                      -> crate::web::app::
crate::error::                    -> crate::web::error::
crate::handlers::                 -> crate::web::handlers::
crate::response::                 -> crate::web::response::

crate::command::                  -> crate::infra::command::
crate::config::                   -> crate::infra::config::
crate::db::                       -> crate::infra::db::
crate::fs_paths::                 -> crate::infra::fs_paths::
crate::http_client::              -> crate::infra::http_client::
crate::logging::                  -> crate::infra::logging::
crate::process::                  -> crate::infra::process::

crate::auth::                     -> crate::domain::auth::
crate::backup::                   -> crate::domain::backup::
crate::dst_map::                  -> crate::domain::map::
crate::game::                     -> crate::domain::game::
crate::models::ClusterRecord      -> crate::domain::cluster::model::ClusterRecord
crate::models::NewCluster         -> crate::domain::cluster::model::NewCluster
```

For repository imports, use the owning domain path:

```text
crate::repositories::cluster::    -> crate::domain::cluster::repository::
crate::repositories::backup::     -> crate::domain::backup::repository::
crate::repositories::mod_info::   -> crate::domain::mods::repository::
crate::repositories::statistics:: -> crate::domain::statistics::repository::
crate::repositories::player_log:: -> crate::domain::statistics::repository::player_log::
crate::repositories::announcement:: -> crate::domain::scheduler::repository::announcement::
crate::repositories::auto_check::   -> crate::domain::scheduler::repository::auto_check::
crate::repositories::tasks::        -> crate::domain::scheduler::repository::tasks::
crate::repositories::kv::           -> crate::domain::admin::repositories::kv::
crate::repositories::web_link::     -> crate::domain::admin::repositories::web_link::
```

- [x] **Step 3: Compile after import rewrite**

Run:

```bash
cargo check
```

Fix only import paths and visibility errors caused by the rewrite.

- [x] **Step 4: Remove root re-export shims from `src/lib.rs`**

Keep only:

```rust
pub mod domain;
pub mod dst;
pub mod infra;
pub mod logs;
pub mod validation;
pub mod web;
```

Keep `pub mod repositories;` and `pub mod models;` only if external integration tests still require those compatibility facades. If they are kept, add comments stating they are compatibility facades.

- [x] **Step 5: Verify no old internal imports remain**

Run:

```bash
rg -n "crate::(app|auth|backup|command|config|db|dst_map|error|fs_paths|game|handlers|http_client|logging|process|response)::" src
```

Expected: no output.

- [x] **Step 6: Full task verification**

Run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
cargo test
```

- [x] **Step 7: Commit**

```bash
git add src tests
git commit -m "refactor: use final module paths"
```

---

### Task 10: Final Documentation, Review, And Release Verification

**Files:**
- Modify: `docs/architecture/rust-module-layout.md`
- Modify: `docs/superpowers/plans/2026-06-18-rust-module-structure-refactor.md`

- [x] **Step 1: Update architecture doc with final state**

Ensure `docs/architecture/rust-module-layout.md` contains:

```markdown
## Final Layout

`web` owns Axum and HTTP response concerns.
`infra` owns external system boundaries.
`domain` owns feature behavior and persistence.
`dst` owns DST file formats and path conventions.

Compatibility facades retained:

- `models`: retained only if integration tests or downstream users import the old model path.
- `repositories`: retained only if downstream users import the old repository path.

Compatibility facades removed:

- root `app`, `auth`, `backup`, `command`, `config`, `db`, `dst_map`, `error`, `fs_paths`, `game`, `handlers`, `http_client`, `logging`, `process`, and `response`, unless Task 9 intentionally kept one with a documented reason.
```

- [x] **Step 2: Run final verification**

Run:

```bash
cargo fmt --all --check
git diff --check
cargo clippy --all-targets --all-features -- -D warnings
go test ./...
cargo test
cargo test --locked
cargo build --release --bin dst-admin-rust
```

Expected: all commands exit 0.

- [x] **Step 3: Start two read-only reviewers**

Reviewer 1 prompt:

```text
Read-only review. Verify the Rust module structure refactor preserved behavior and made the layout clearer. Focus on route behavior, JSON shape, binary target, and whether compatibility facades were kept or removed intentionally. Return PASS or CHANGES_REQUESTED with file:line findings.
```

Reviewer 2 prompt:

```text
Read-only review. Verify code quality and maintainability after the Rust module structure refactor. Focus on cyclic dependencies, misplaced domain/infra/web code, visibility leaks, and oversized files that should have been split. Return PASS or CHANGES_REQUESTED with file:line findings.
```

- [x] **Step 4: Fix reviewer findings**

Only fix Critical or Important findings before closing the refactor. Minor naming/doc cleanup can be committed if it is low risk.

- [x] **Step 5: Commit final documentation**

```bash
git add docs/architecture/rust-module-layout.md docs/superpowers/plans/2026-06-18-rust-module-structure-refactor.md
git commit -m "docs: finalize rust module layout"
```

---

## Estimated Execution

Recommended execution mode: subagent-driven, one worker per task, with main-agent review between tasks.

Expected size:

- Tasks 1-3: 1 short execution block.
- Tasks 4-5: 1 medium execution block.
- Task 6: 1 medium execution block.
- Task 7: 1 medium execution block.
- Task 8: 1-2 larger execution blocks because it splits the largest files.
- Tasks 9-10: 1 final execution block.

Total estimate: 6-7 execution blocks for a careful refactor with review checkpoints.

## Acceptance Criteria

- `cargo build --release --bin dst-admin-rust` produces the same binary target name.
- `cargo test`, `cargo test --locked`, `cargo clippy --all-targets --all-features -- -D warnings`, and `go test ./...` all pass.
- No route disappears from `compat_manifest_tests`.
- No response JSON shape changes in existing integration tests.
- No command strings are reintroduced; command execution remains argv-based.
- No symlink/path traversal hardening is weakened.
- New module layout is visible from `src/lib.rs`: `web`, `infra`, `domain`, `dst`, `logs`, and `validation`.
- Large migration-era files are either split or documented as intentionally retained.
