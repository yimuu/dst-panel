# DST Admin Go to Rust Migration Design

## Goal

Migrate the Go backend of this Don't Starve Together server panel to a Rust 2024 backend inside the current repository. The Rust backend must preserve the existing user-facing deployment shape, API behavior, data files, and DST management workflows while replacing the Go implementation.

The target release binary is `dst-admin-rust`.

## Repository Strategy

Use an in-place migration in the current repository.

The Rust project will be added at the repository root with `Cargo.toml` and `src/`, using Rust edition `2024`. Existing Go code remains during migration as the reference implementation and compatibility oracle. After the Rust backend reaches API and behavior parity, the Go backend directories and files can be removed.

Keep these existing assets:

- `config.yml`
- `dst_config`
- `static/`
- `dist/` when present
- Docker scripts and install scripts, updated to start `dst-admin-rust`
- documentation and screenshots
- existing SQLite database file name `dst-db`
- existing `password.txt` format

Remove Go only at the final cleanup phase, after Rust coverage is verified:

- `main.go`
- `go.mod`
- `go.sum`
- `api/`
- `bootstrap/`
- `collect/`
- `config/` Go package files
- `constant/`
- `middleware/`
- `model/`
- `router/`
- `schedule/`
- `service/`
- `utils/`
- `vo/`

## Runtime Shape

The Rust application starts from a single binary named `dst-admin-rust`. It reads `config.yml`, initializes logging, opens or creates `dst-db`, starts background collectors and schedulers, and serves HTTP on `bindAddress:port`.

Docker and shell entrypoints will be updated to run:

```bash
exec ./dst-admin-rust
```

The Rust application should not panic when optional frontend assets such as `dist/` are missing. Static routes should serve files when present and return standard missing-file responses when absent.

## Rust Stack

Use these core libraries unless implementation discovery shows a concrete reason to adjust:

- `axum` for HTTP routing, extractors, JSON, SSE, WebSocket, and static serving.
- `tokio` for the async runtime, timers, file IO where useful, and process management.
- `sqlx` with SQLite for database access and explicit schema management.
- `serde`, `serde_json`, and `serde_yaml` for DTOs and configuration.
- `tower` and `tower-http` for middleware, compression, tracing, and static files.
- `tower-sessions` or a scoped equivalent for session handling.
- `tracing` and `tracing-subscriber` for console and file logging.
- `chrono` for timestamps.
- `uuid` for generated identifiers.
- `reqwest` for third-party HTTP calls.
- `tokio-tungstenite` or axum WebSocket support for `/ws`.
- `notify` or a small tail implementation for log following.
- `zip` for backup archive handling.
- An INI parser crate for `cluster.ini` and `server.ini`.
- A Lua parser or embedded Lua crate for the existing Lua data files and mod metadata.

## Code Documentation and Logging

The Rust implementation must include complete, useful comments and structured logs.

Comment requirements:

- Every public module must start with a short `//!` module comment that explains the module's responsibility.
- Public structs, enums, traits, and functions that are part of the backend's internal API must have `///` doc comments.
- Complex compatibility behavior must have focused inline comments explaining the Go behavior being preserved.
- External command construction, path validation, SQLite schema compatibility, session behavior, and Lua/INI parsing must be commented where future maintainers could otherwise misread the intent.
- Comments must explain why code exists or why compatibility matters; they must not restate obvious assignments.

Logging requirements:

- Use `tracing` for structured logs.
- Log application startup, effective bind address, config file loading, database opening, migration start/end, static asset availability, and background task startup/shutdown.
- Log authentication failures, direct-login decisions from `whiteadminip`, API validation failures, and non-sensitive request context for operational debugging.
- Log external command intent, sanitized arguments, exit status, stderr summaries, and timeout/kill behavior.
- Log scheduler and auto-check task creation, execution start/end, skip reasons, and failures.
- Log file upload/download/backup operations with sanitized paths and sizes.
- Do not log secrets, passwords, session values, cluster tokens, Steam API keys, or raw user-provided command bodies.

## Compatibility Requirements

### Configuration

Rust must read the existing `config.yml` keys and preserve Go defaults:

- `bindAddress`
- `port`
- `path`
- `database`
- `steamcmd`
- `steamAPIKey`
- `flag`
- `wanip`
- `whiteadminip`
- `token`
- `dstVersionUrl`
- `autoUpdateModinfo.enable`
- `autoUpdateModinfo.checkInterval`
- `autoUpdateModinfo.updateCheckInterval`
- `dstCliPort`

Default values must match the current Go behavior:

- `autoUpdateModinfo.updateCheckInterval`: `10` when missing or zero.
- `autoUpdateModinfo.checkInterval`: `5` when missing or zero.
- `dstVersionUrl`: `https://api.dstserverlist.top/api/v2/Server/Version` when missing.

The current sample config includes `autoCheck`, but the Go `Config` struct does not parse it. Rust should preserve the key when rewriting config in the future, but the initial backend migration does not need to depend on it.

### Database

Rust must continue using the SQLite file configured by `database`, usually `dst-db`.

The migration should create or preserve tables equivalent to the current GORM models:

- `spawns`
- `player_logs`
- `connects`
- `regenerates`
- `mod_infos`
- `clusters`
- `job_tasks`
- `auto_checks`
- `announces`
- `web_links`
- `backup_snapshots`
- `log_records`
- `kvs`

The Rust schema must preserve GORM-style metadata columns for compatibility:

- `id`
- `created_at`
- `updated_at`
- `deleted_at`

Schema changes must be additive during migration. Do not rename tables, remove columns, or change JSON/text payload formats in the first Rust-compatible release.

### Authentication

Rust must preserve the existing `password.txt` format:

```text
username=admin
password=123456
displayName=admin
photoURL=xxx
```

The auth middleware must preserve the current behavior:

- Requests outside `/api` are public.
- `/api/login`, `/api/logout`, `/ws`, `/api/bootstrap`, `/api/init`, and `/api/install/steamcmd` are public.
- Other `/api` requests require a valid session unless `whiteadminip` allows direct login.
- Unauthorized API requests return HTTP 401.

### API

The Rust backend must keep existing paths, methods, query parameters, JSON field names, and response envelope semantics. Most successful business responses use:

```json
{
  "code": 0,
  "msg": "",
  "data": {}
}
```

Existing login-style responses use their current shape and status semantics, including `code: 200` for login success.

Compatibility routes include these groups:

- init and install: `/api/init`, `/api/install/steamcmd`
- auth: `/api/login`, `/api/logout`, `/api/change/password`, `/api/user`
- cluster collection routes: `/api/cluster`
- DST config and KV: `/api/dst/config`, `/api/kv`
- game console and status: `/api/game/*`
- level management: `/api/cluster/level`, `/api/game/8level/*`
- players and player logs: `/api/game/player/*`, `/api/player/log`
- backups: `/api/game/backup/*`
- mods: `/api/mod/*`
- task scheduling: `/api/task/*`
- statistics: `/api/statistics/*`
- logs: `/api/game/log/stream`, `/ws`
- auto check: `/api/auto/check2`
- web links and webhook: `/api/web/link`, `/webhook`
- third-party proxy: `/api/dst/version`, `/api/dst/home/*`, `/api/dst-static/*filepath`, `/steam/dst/news`
- map generation: `/api/dst/map/*`
- file upload/background: `/api/file/*`
- sharing: `/api/share/*`, `/share/cluster`

## Module Design

Rust modules should follow business boundaries rather than copying every Go package literally.

### `config`

Reads `config.yml`, applies defaults, exposes runtime settings, and defines repository-relative paths.

### `db` and `models`

Own SQLite pool creation, migration, row structs, and repository functions. Use explicit SQL and typed DTOs. Avoid hidden global mutable database state.

### `http`

Builds the axum router, middleware stack, compression, sessions, error mapping, JSON response helpers, SSE, WebSocket routes, and static file routing.

### `auth`

Parses and writes `password.txt`, handles login/logout/password changes, user info, session lookup, and `whiteadminip` checks.

### `dst`

Owns DST filesystem paths and parsing/writing of:

- `cluster.ini`
- `cluster_token.txt`
- `adminlist.txt`
- `blocklist.txt`
- `whitelist.txt`
- `server.ini`
- `leveldataoverride.lua`
- `modoverrides.lua`
- `dedicated_server_mods_setup.lua`

### `process`

Owns process execution, Linux `screen` integration, Windows process support, DST startup commands, graceful shutdown, hard kill fallback, and command validation.

User-provided identifiers such as cluster names, level names, mod IDs, and file names must not be inserted into shell strings directly. Prefer `Command` with argument arrays. Where `screen` requires string commands, validate inputs centrally before invoking.

### `game`

Implements update, start, stop, status, broadcast, player operations, rollback, regenerate, clean world, archive metadata, and system information.

### `cluster`

Implements cluster CRUD, default cluster creation, and mapping between database cluster records and DST filesystem layout. HTTP cluster CRUD is not part of the foundation slice because Go's create path performs DST installation/world initialization side effects and Go's list path returns a paginated runtime view object.

### `backup`

Implements backup listing, creation, upload, download, delete, rename, restore, and snapshot scheduling.

### `mod_service`

Implements Steam Workshop details lookup, installed mod listing, modinfo parsing/writing, UGC file upload/delete, and update checks.

### `logs`

Implements log snapshot, tail-follow streaming, SSE responses, WebSocket tailing, and player activity collection from DST logs.

### `schedule`

Implements cron job persistence and execution for backup, update, start, stop, restart, regenerate, game start, game stop, and announcement tasks.

### `auto_check`

Implements periodic checks for level down, mod update, and game version update, with announcement behavior matching the Go service.

### `third_party`

Implements DST server list calls, lobby detail calls, static proxying, public IP lookup, and Steam news.

### `map`

Implements DST map image generation and session file helpers.

## External Command Safety

The Go backend uses shell string concatenation in several places. The Rust migration should treat that behavior as compatibility requirements plus a security improvement opportunity.

Rules:

- Use `tokio::process::Command` or `std::process::Command` with explicit arguments whenever possible.
- Centralize all `screen` commands in `process`.
- Validate cluster names, level names, file names, KU IDs, and mod IDs before they reach command execution.
- Reject path traversal in upload/download/file APIs.
- Use allowlisted command templates for DST operations.
- Log command intent and exit status, but do not log secrets such as cluster tokens.

## Migration Phases

### Phase 1: Rust Foundation

Create the Rust project, config loading, logging, SQLite pool, schema migration, response helpers, auth/session middleware, health route, and static file serving.

The application should run locally with:

```bash
cargo run --bin dst-admin-rust
```

### Phase 2: Low-Risk API Parity

Migrate APIs that do not start external DST processes:

- login/logout/user/password
- init state checks
- KV
- web links
- cluster schema/repository compatibility only; HTTP CRUD remains an explicit 501 stub until the cluster side-effect slice
- DST config file read/write helpers
- player list files
- backup listing metadata
- static assets

### Phase 3: Logs, Streams, and WebSocket

Migrate:

- `/api/game/log/stream`
- `/ws`
- server log reads/downloads
- player log collection and query APIs

### Phase 4: DST Process Control

Migrate:

- start/stop/status
- console commands
- broadcast/player operations
- update game
- system info
- Windows path support where feasible

### Phase 5: Mods, Third-Party APIs, Map, Backup Restore

Migrate:

- Steam Workshop APIs
- mod download/update/config
- third-party proxy APIs
- map generation
- backup upload/download/restore
- snapshot backup scheduling

### Phase 6: Scheduling and Auto Check

Migrate cron tasks and automatic level/mod/game update checks.

### Phase 7: Release and Go Removal

Update:

- `build_linux.sh`
- `build_window.sh`
- `Dockerfile`
- `docker-entrypoint.sh`
- README files

Then remove Go backend files only after compatibility tests pass and the Rust binary can replace the Go binary for normal use.

## Testing Strategy

### Unit Tests

Cover:

- `config.yml` parsing and defaulting.
- `password.txt` parsing/writing.
- path construction for Klei/DST directories.
- INI parsing/writing.
- Lua table parsing for supported DST files.
- response envelope helpers.
- command input validation.

### Integration Tests

Use temporary directories and temporary SQLite databases to cover:

- app startup without `dist/`.
- login and session-protected routes.
- `whiteadminip` direct login.
- KV CRUD.
- web link CRUD.
- cluster repository CRUD plus HTTP 501 coverage for `/api/cluster` until side effects are migrated.
- static missing-file behavior.
- backup list against a temporary backup directory.
- log snapshot and streaming setup without requiring a real DST server.

### Compatibility Tests

Build route and response parity tests against the Go implementation where practical. At minimum, maintain a compatibility manifest of route method/path pairs and key response shapes. Rust tests should assert the manifest routes exist and return compatible status/envelope behavior.

External Steam and DST operations should use command mocks by default. Tests must not download SteamCMD, install DST, or launch a real game server unless explicitly run as manual end-to-end tests.

## Success Criteria

The migration is successful when:

- `cargo test` passes.
- `cargo build --release --bin dst-admin-rust` succeeds.
- `dst-admin-rust` starts with the existing `config.yml`.
- Existing `dst-db` data remains readable.
- Existing `password.txt` credentials work.
- Existing frontend API calls continue to work for the migrated route set.
- Docker starts `dst-admin-rust`.
- Go backend files are removed only after Rust parity is verified.

## Non-Goals

Do not rewrite the frontend as part of this migration.

Do not change the user-facing API contract unless a later task explicitly authorizes a breaking versioned API.

Do not change the DST save/config file formats.

Do not make real SteamCMD or DST downloads part of automated tests.
