# Frontend React Rebuild Design

## Goal

Rebuild `web-ui` as a React + TypeScript + Vite frontend that closely matches the official DST Admin Go preview UI while preserving this repository's Rust backend API contracts and static serving boundary.

The previous Vue rebuild produced a runnable frontend, but the official preview is built with React, Ant Design, Ant Design Pro-style components, Monaco Editor, Vite, and mocked preview data. Because the new goal is visual and interaction parity with the official preview, React is the better fit than continuing to emulate Ant Design Pro with Vue.

Reference sources:

- Official preview: `https://carrot-hu23.github.io/dst-admin-go-preview/`
- Preview static repository: `https://github.com/carrot-hu23/dst-admin-go-preview`
- Main project repository: `https://github.com/carrot-hu23/dst-admin-go`
- Local target screenshots: `docs/image/dashboard.png`, `docs/image/panel.png`, `docs/image/home.png`, `docs/image/level.png`, `docs/image/mod1.png`, `docs/image/mod2.png`, `docs/image/mod3.png`, `docs/image/player.png`, `docs/image/playerlog.png`, `docs/image/lobby.png`, `docs/image/selectormod.png`, `docs/image/toomanyitemplus.png`

## Non-Goals

- Do not change Rust backend routes, request parameters, response envelopes, cookies, stream paths, or static file serving behavior.
- Do not introduce i18n. This project remains Chinese-only.
- Do not keep Vue, Element Plus, Pinia, or Vue Router in the rebuilt frontend.
- Do not embed the official preview's minified React bundle as the application source.
- Do not reverse engineer private source maps. The preview has no published source map, so visual parity must come from screenshots, runtime behavior, static assets, API contracts, and maintainable React code.
- Do not attempt pixel-perfect completion of every page in a single unreviewed commit. Rebuild in verifiable batches.

## Repository Strategy

Keep the semantic frontend root as `web-ui` so backend build and deployment paths stay stable:

```text
dst-panel/
├── src/              # Rust backend
├── tests/            # Rust tests
├── static/           # DST templates and runtime scripts
├── dist/             # Built frontend assets served by Rust
├── web-ui/           # React + TypeScript + Vite frontend source
│   ├── public/       # Static assets copied into dist
│   ├── src/          # React application source
│   ├── package.json
│   ├── package-lock.json
│   ├── tsconfig*.json
│   └── vite.config.ts
├── Cargo.toml
└── Cargo.lock
```

Use Vite's latest React TypeScript scaffold in a temporary directory, then replace `web-ui`'s base project files from the scaffold. This avoids carrying Vue-specific toolchain residue into the React build.

`web-ui` must continue building to root `../dist`. Root `dist/` remains tracked because the Rust binary serves it.

## Version Policy

Use latest stable npm packages available at rebuild time and commit `package-lock.json` for reproducibility.

Known latest versions checked on June 27, 2026:

- `react@19.2.7`
- `react-dom@19.2.7`
- `react-router@8.0.1`
- `vite@8.1.0`
- `@vitejs/plugin-react@6.0.3`
- `typescript@6.0.3`
- `antd@6.4.5`
- `@ant-design/icons@6.3.1`
- `@ant-design/pro-components@2.8.10`
- `@tanstack/react-query@5.101.1`
- `axios@1.18.1`
- `monaco-editor@0.55.1`
- `@monaco-editor/react@4.7.0`
- `vitest@4.1.9`
- `@testing-library/react@16.3.2`
- `@testing-library/jest-dom@6.9.1`
- `eslint@10.6.0`
- `prettier@3.8.5`

If any listed latest package has a peer dependency conflict with the scaffold, resolve by selecting the newest compatible stable version and document the exception in the implementation plan.

## Frontend Stack

Use this stack:

- React with function components and hooks
- TypeScript
- Vite
- React Router for hash routes
- Ant Design for base components
- `@ant-design/pro-components` for ProLayout, ProCard, ProTable or ProForm where it fits
- `@ant-design/icons` for menu and action icons
- TanStack Query for server data fetching, mutation, loading, retry, and cache invalidation
- A small local store only for cross-cutting UI/auth state that does not belong in query cache
- Axios for HTTP calls and file upload/download handling
- Monaco Editor through `@monaco-editor/react`
- Vitest and Testing Library for unit/component tests

The application should not use a global design framework abstraction that hides Ant Design. Shared wrappers are acceptable only where they preserve official preview behavior, such as page cards, console/log panels, API error display, and protected route handling.

## Routing

Use hash history so the Rust backend does not need SPA fallback changes:

```text
/#/login
/#/init
/#/dashboard
/#/panel
/#/home/clusterIni
/#/home/adminlist
/#/home/whitelist
/#/home/blacklist
/#/levels/levels
/#/levels/selectorMod
/#/levels/preinstall
/#/levels/genMap
/#/mod
/#/backup
/#/playerLog
/#/setting
/#/lobby
/#/help
/#/userProfile
```

Default route should redirect to `/#/panel` unless the app initialization/auth guard redirects to `/#/init` or `/#/login`.

## Source Layout

Use feature-oriented source boundaries:

```text
web-ui/src/
├── app/
│   ├── App.tsx
│   ├── main.tsx
│   ├── providers.tsx
│   └── router.tsx
├── layouts/
│   ├── AdminLayout.tsx
│   ├── AuthLayout.tsx
│   ├── AppHeader.tsx
│   └── menu.tsx
├── pages/
│   ├── LoginPage.tsx
│   ├── InitPage.tsx
│   ├── DashboardPage.tsx
│   ├── PanelPage.tsx
│   ├── ClusterIniPage.tsx
│   ├── PlayerListPage.tsx
│   ├── WorldLevelsPage.tsx
│   ├── WorldModSelectionPage.tsx
│   ├── PreinstallPage.tsx
│   ├── MapPreviewPage.tsx
│   ├── ModPage.tsx
│   ├── BackupPage.tsx
│   ├── PlayerLogPage.tsx
│   ├── SettingsPage.tsx
│   ├── LobbyPage.tsx
│   ├── HelpPage.tsx
│   └── UserProfilePage.tsx
├── features/
│   ├── auth/
│   ├── backups/
│   ├── clusters/
│   ├── dashboard/
│   ├── game/
│   ├── levels/
│   ├── maps/
│   ├── mods/
│   ├── panel/
│   ├── room/
│   ├── settings/
│   └── statistics/
├── shared/
│   ├── api/
│   ├── config/
│   ├── hooks/
│   ├── styles/
│   ├── types/
│   └── ui/
└── test/
    ├── setup.ts
    └── *.test.tsx
```

`pages/` owns route-level composition. `features/` owns domain API clients, domain transforms, and feature-specific components. `shared/` owns infrastructure and generic UI helpers.

## Static Assets

Keep `web-ui/public` as the direct-copy asset root. Reuse existing assets already collected from the preview:

- `favicon.ico`
- `assets/light-bg.png`
- `assets/dark-bg.png`
- `assets/pig.gif`
- `assets/fish.gif`
- `assets/dst-emoji.woff2`
- `assets/dst/*`
- `misc/*`

The official preview static repository also contains additional useful files such as `misc/level_master.jpg`, `misc/level_caves.jpg`, `misc/MasterCaves.json`, and donation/ad images. Add missing assets only when a page needs them.

Do not copy hashed JavaScript or CSS chunks from the official preview into source. Use them only as analysis references.

## Visual Design Targets

Global shell:

- White sidebar with `Dst-admin-go` brand, `v1.6.1` tag, icon-only collapse trigger, and Ant Design menu states.
- White top header with small utility icons, theme control, avatar circle, and username.
- Light gray application background.
- Page content in white rounded ProCard-style containers with 16px radius.
- Primary color aligned with official preview's blue/purple Ant Design tone.
- Operational density similar to screenshots: compact but readable forms, tables, tabs, alerts, and action rows.

High-priority pages:

1. Dashboard: date range toolbar, weekly/monthly selector, statistic cards, active player chart, role donut chart, top-player and reset timeline cards.
2. Panel: tabs for panel/remote/TooManyItemsPlus/custom commands, system resource summary, server info, server log console, world list, player list, command input, save/rollback actions.
3. Room settings: large horizontal Ant Design form matching `docs/image/home.png`, fixed bottom save action, room/admin/white/black list menu grouping.
4. World settings: level tabs, nested tab groups, alert, world setting grid with DST images, save/add/import/download actions.
5. Mod settings: setting/subscribe/UGC tabs, alert, toolbar, left mod list, right selected mod details, bottom action bar.

Secondary pages:

- Backup: archive list, snapshot settings, upload/create/restore/delete/rename actions.
- Player log: filterable table and block/delete actions.
- Lobby: server lobby table/list with query controls.
- Settings: DST config paths, scheduled task/web link settings if supported by current APIs.
- Help: markdown/help content from `public/misc`.
- Login/init/profile: Ant Design versions that visually fit the official shell.

## API Layer

Recreate the current frontend API layer semantics in React-friendly TypeScript:

```text
shared/api/http.ts
shared/api/envelope.ts
shared/api/types.ts
features/auth/auth.api.ts
features/backups/backup.api.ts
features/clusters/cluster.api.ts
features/game/game.api.ts
features/levels/level.api.ts
features/maps/map.api.ts
features/mods/mod.api.ts
features/room/room.api.ts
features/settings/settings.api.ts
features/statistics/statistics.api.ts
```

Rules:

- Preserve backend paths exactly.
- Use same-origin base URL in production.
- Dev proxy forwards `/api`, `/ws`, `/steam`, `/webhook`, `/share`, `/assets`, and `/misc` as needed.
- Include cookies by default.
- Keep response envelope helpers: read data, assert success, extract error message.
- File downloads return `Blob`.
- Uploads use `FormData`.
- Query/mutation keys must be explicit and feature-scoped.

## Auth And Guards

The React router must preserve the current first-run and auth behavior:

- Check `/api/init` before protected routes.
- Redirect to `/#/init` if first-run setup is required.
- Redirect authenticated users away from init/login where appropriate.
- Fetch `/api/user` before entering protected routes when auth state is unknown.
- On logout or 401, clear auth state and route to login.

## Testing Strategy

Use test-driven implementation for behavior that can regress:

- API envelope helpers
- API clients request paths and payloads
- Auth route guard decisions
- Menu flattening and active route labels
- Panel/world/mod action transforms
- Settings/profile validation helpers
- Page smoke mounting for all routes

Visual parity must be checked with browser screenshots after each major UI batch. Exact screenshot diffing is optional in the first React batch, but manual screenshot review against `docs/image/*.png` is required before claiming a page is visually aligned.

Required verification before completion:

```bash
npm run test:unit -- --run
npm run type-check
npm run lint:check
npm run format:check
npm run build
```

Because root `dist/` is tracked, successful implementation must include regenerated root `dist/index.html` and hashed assets.

## Rollout Plan

Implement in batches:

1. Scaffold React/Vite/TS and remove Vue toolchain.
2. Add Ant Design Pro shell, router, auth guard, shared API layer, and placeholder pages.
3. Rebuild Dashboard and Panel to match official preview structure.
4. Rebuild Room settings and player list pages.
5. Rebuild World settings, selector mod, preinstall, and map preview pages.
6. Rebuild Mod settings.
7. Rebuild Backup, Player log, Settings, Lobby, Help, Login, Init, and Profile pages.
8. Final visual QA, root `dist/` refresh, and full verification.

Each batch should keep the app runnable and should have focused tests. If a page has backend data gaps, use realistic empty/loading/error states that preserve the official layout instead of inventing unsupported backend calls.

## Success Criteria

- `web-ui` is a clean React + TypeScript + Vite project created from a modern scaffold.
- Vue, Element Plus, Pinia, and Vue Router are removed from runtime dependencies.
- Ant Design and Ant Design Pro components define the UI language.
- Core routes load through the same hash paths used by the official preview.
- The global shell visually matches the official preview's sidebar/header/content structure.
- Dashboard, Panel, Room, World, and Mod pages are substantially aligned with official screenshots.
- Existing Rust backend API contracts remain unchanged.
- Root `dist/` is regenerated from the React project.
- Full frontend verification and relevant Rust static serving checks pass.
