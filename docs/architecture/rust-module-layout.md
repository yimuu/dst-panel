# Rust Module Layout

The Rust backend uses feature-first modules with three root groups and a
separate DST format module. The layout is intentionally explicit so ownership is
visible from `src/lib.rs`.

## Final Layout

`web` owns Axum and HTTP response concerns.
`infra` owns external system boundaries.
`domain` owns feature behavior and persistence.
`dst` owns DST file formats and path conventions.

Root modules exported by `src/lib.rs`:

- `web`: HTTP application assembly, route handlers, response envelopes, and web errors.
- `infra`: commands, HTTP clients, process snapshots, database connection, config loading, logging, and safe filesystem primitives.
- `domain`: DST admin business domains such as cluster, game lifecycle, backup, mods, map, auth, scheduler, admin settings, and statistics.
- `dst`: DST file formats and path conventions shared by multiple domains.
- `logs`: log tailing primitives shared by web routes and domain queries.
- `validation`: user-input validators shared across route and domain boundaries.

Compatibility facades retained:

- None. `models` and `repositories` were removed after integration tests moved to
  the owning domain paths.

Compatibility facades removed:

- root `app`, `auth`, `backup`, `command`, `config`, `db`, `dst_map`, `error`,
  `fs_paths`, `game`, `handlers`, `http_client`, `logging`, `process`, and
  `response`.
- root `models` and `repositories`.

No route behavior, JSON response shape, database schema, command argv
construction, or safety hardening changed during this refactor.
