# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

DST Admin Rust is a Rust 2024 web management panel for "Don't Starve Together" dedicated servers. It preserves the legacy panel API and deployment behavior while replacing the backend with Rust. The target binary is `dst-admin-rust`.

## Development Commands

### Running

```bash
cargo run --bin dst-admin-rust
```

The server listens on port `8082` by default. Runtime configuration is loaded from `config.yml`.

### Building

```bash
./build_linux.sh
./build_window.sh
```

`./build_linux.sh` copies `target/<target>/release/dst-admin-rust` to `./dst-admin-rust`. `./build_window.sh` copies the Windows executable to `./dst-admin-rust.exe`.

### Testing

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
```

Use focused integration tests while iterating, for example:

```bash
cargo test --test scheduler_auto_check_tests
cargo test --test release_entrypoint_tests
```

## Repository Layout

```text
dst-admin-go/
├── src/
│   ├── domain/          # Business domains: auth, backup, game, scheduler, admin
│   ├── dst/             # DST file/path/config helpers
│   ├── infra/           # Config, database, process, command, logging, filesystem primitives
│   ├── logs/            # Legacy-compatible log parsing
│   └── web/             # Axum application, routes, handlers, response/error types
├── tests/               # Integration and compatibility tests
├── static/              # DST templates and runtime install scripts
├── dist/                # Minimal static app shell for clean Docker builds
├── scripts/             # Release/Docker/helper scripts
├── config.yml           # Default application configuration
├── Cargo.toml
└── Cargo.lock
```

## Architecture

The Rust backend uses a layered structure:

1. `web`: HTTP routing, request/response shapes, authentication middleware, static files.
2. `domain`: feature behavior and compatibility logic for game management, backups, scheduler, mods, players, logs, and admin data.
3. `infra`: external boundaries such as SQLite, command execution, process snapshots, config loading, logging, and safe filesystem access.

Prefer existing local helpers and patterns when adding behavior. Keep route payloads and filesystem/database compatibility aligned with the legacy panel unless a deliberate migration step says otherwise.

## Configuration

The default `config.yml` includes:

```yaml
bindAddress: ""
port: 8082
dataDir: "./data"
dstCliPort: 8102
database: dst-db
dstVersionUrl: "https://api.dstserverlist.top/api/v2/Server/Version"
```

`dataDir` is intentionally supported for compatibility. With the default config the SQLite database path is `./data/dst-db`.

## Runtime Notes

- SQLite schema migration is handled by `src/infra/db.rs` and preserves known legacy tables/columns.
- Static frontend files are served from `dist/`; missing assets return normal 404s in development.
- Scheduler jobs are stored in `job_tasks` and executed by `src/domain/scheduler/runtime.rs`.
- The scheduler supports legacy cron descriptors, `@every` durations, `CRON_TZ=`/`TZ=`, and robfig-compatible day-of-month/day-of-week behavior.
- Linux game process control uses safe argv command construction and `screen` sessions.
- Some legacy names, such as `dst-admin-go.log`, are retained intentionally for compatibility.

## Development Guidelines

- Use `rg` for code search.
- Add focused tests for behavioral changes before editing implementation.
- Keep comments concise and useful; prefer tracing logs at operational boundaries.
- Do not reintroduce removed backend files or module metadata.
- Preserve `static/customcommands.lua`, `static/script/install_steamcmd.sh`, and release scripts unless a migration task explicitly replaces them.
- Before merging, run the full verification set listed in Testing plus a release binary build.
