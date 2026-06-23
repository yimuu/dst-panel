# Vue Frontend Rebuild Design

## Goal

Create a maintainable Vue 3 frontend source project for DST Admin Rust, using the current `dist/` bundle and Rust API surface as the compatibility reference. The first deliverable is a runnable Vue + TypeScript + Vite application with a clear directory architecture, shared API layer, authenticated admin shell, menu structure, and core page skeletons for login, panel, worlds, mods, backups, and settings.

This rebuild is not a byte-for-byte decompilation of the current frontend bundle. The current `dist/` artifact was built with React 18, React Router, Ant Design, Ant Design ProComponents, i18next, Zustand, Monaco Editor, and Vite. The new source project intentionally uses Vue while preserving user-facing workflows, routes, backend API calls, and static asset behavior.

## Non-Goals

- Do not redesign the Rust backend module layout.
- Do not change Rust API paths, request parameters, response envelopes, or authentication behavior.
- Do not replace the existing `dist/` static serving contract in this phase.
- Do not fully reimplement every detailed page interaction in the first increment.
- Do not introduce a monorepo package manager workspace unless future frontend package sharing creates a concrete need.

## Repository Strategy

Add a semantic frontend source directory at the repository root:

```text
dst-panel/
├── src/              # Existing Rust backend
├── tests/            # Existing Rust integration tests
├── static/           # DST templates and runtime scripts
├── dist/             # Built frontend assets served by Rust
├── web-ui/           # New Vue 3 + TypeScript + Vite frontend source
│   ├── public/       # Files copied directly into dist
│   ├── src/          # Vue application source
│   ├── package.json
│   ├── package-lock.json
│   ├── tsconfig*.json
│   └── vite.config.ts
├── Cargo.toml
└── Cargo.lock
```

Keep the Rust backend structure unchanged. The backend already serves `dist/index.html`, `/assets/*`, `/misc/*`, `/favicon.ico`, and legacy static prefixes. Reusing this deployment boundary avoids coupling frontend source organization to Rust internals.

`web-ui` builds to `../dist`. During development, Vite proxies API and stream requests to the Rust server on port `8082`; during production, the Rust binary serves the built assets from `dist/`.

## Version Policy

Use current npm `latest` versions as of June 23, 2026:

- `vue@3.5.38`
- `vite@8.1.0`
- `@vitejs/plugin-vue@6.0.7`
- `create-vue@3.22.4`

Create and commit `package-lock.json` so future builds are reproducible even though the initial scaffold uses latest packages. Use exact installed versions in `package-lock.json`; keep semver ranges in `package.json` conventional unless the scaffold emits exact pins.

## Frontend Stack

Use this stack for the first increment:

- Vue 3 with Single File Components and `<script setup lang="ts">`
- TypeScript with `vue-tsc` command-line checking
- Vite for development server and production builds
- Vue Router for client-side routes
- Pinia for shared state
- Axios for HTTP calls
- Element Plus for the admin UI component system
- `@element-plus/icons-vue` for UI icons
- `vue-i18n` for language resources
- Monaco Editor integration points for Lua/config editing screens
- Vitest and Vue Test Utils for unit/component tests
- ESLint and Prettier if selected by `create-vue`

Element Plus is the preferred Vue UI library because this panel is an operational admin application: tables, forms, dialogs, tabs, upload controls, switches, cards, descriptions, notifications, and menus are first-class use cases. It also maps well to the existing Ant Design shaped bundle without forcing a React compatibility layer.

Pinia is preferred over Vuex because it is the current Vue state management recommendation for new Vue 3 projects and works cleanly with Composition API and TypeScript.

Axios is preferred over raw `fetch` for this project because the existing backend uses many query/header variations, file upload/download flows, binary responses, and common response envelopes. A single Axios instance with interceptors keeps cookie/session handling, error normalization, and `Cluster` headers consistent.

## Application Routes

The Vue app uses browser history only if the Rust fallback supports route refresh for all frontend paths. In the first increment, use hash history to avoid changing backend fallback routing:

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

This preserves direct access through `dist/index.html` without requiring a Rust catch-all route for every client path. A subsequent phase can switch to history mode if the backend adds a safe SPA fallback.

## Source Layout

Use feature-oriented boundaries under `web-ui/src`:

```text
web-ui/src/
├── app/
│   ├── main.ts
│   ├── router.ts
│   └── providers.ts
├── layouts/
│   ├── AdminLayout.vue
│   ├── AuthLayout.vue
│   └── menu.ts
├── pages/
│   ├── LoginPage.vue
│   ├── InitPage.vue
│   ├── DashboardPage.vue
│   ├── PanelPage.vue
│   ├── WorldLevelsPage.vue
│   ├── ModPage.vue
│   ├── BackupPage.vue
│   ├── SettingsPage.vue
│   ├── PlayerLogPage.vue
│   ├── LobbyPage.vue
│   ├── HelpPage.vue
│   └── UserProfilePage.vue
├── features/
│   ├── auth/
│   ├── clusters/
│   ├── game/
│   ├── levels/
│   ├── mods/
│   ├── backups/
│   ├── settings/
│   └── statistics/
├── shared/
│   ├── api/
│   ├── assets/
│   ├── components/
│   ├── composables/
│   ├── config/
│   ├── i18n/
│   ├── stores/
│   ├── styles/
│   └── types/
└── vite-env.d.ts
```

`pages/` owns route-level composition. `features/` owns domain API wrappers, domain stores, and feature-specific components. `shared/` owns reusable infrastructure and generic UI. This keeps core page skeletons easy to add now while leaving room to deepen each workflow in subsequent increments.

## Static Assets

Seed `web-ui/public` from current production assets needed by the first increment:

- `favicon.ico`
- `misc/` JSON, markdown, and image files used by help/preinstall/world settings
- `assets/dst/` DST character and UI images
- `assets/login.png`
- `assets/light-bg.png`
- `assets/dark-bg.png`
- `assets/pig.gif`
- `assets/fish.gif`
- `Dst Emoji.woff2` or equivalent font assets

Do not manually copy hashed JavaScript or CSS chunks from the current `dist/assets`; those are old build artifacts. The new build will emit its own chunks.

If current bundle references missing assets such as donation images, `wonkey_inv.png`, or `dst_button_normal.png`, treat them as optional and use safe fallbacks in the first increment unless the file already exists in `dist`.

## API Layer

Create one Axios client in `shared/api/http.ts`:

- Base URL defaults to same-origin.
- Dev proxy forwards `/api`, `/ws`, `/steam`, `/webhook`, and `/share` to `http://127.0.0.1:8082`.
- Requests include cookies by default.
- A request helper can attach the current `Cluster` header from Pinia.
- Responses are normalized into a common `ApiEnvelope<T>` type.
- HTTP 401 clears auth state and routes to login.
- File download helpers return `Blob`.
- Upload helpers use `FormData`.

Define feature clients around backend route groups:

```text
shared/api/http.ts
shared/api/types.ts
features/auth/auth.api.ts
features/clusters/cluster.api.ts
features/game/game.api.ts
features/levels/level.api.ts
features/mods/mod.api.ts
features/backups/backup.api.ts
features/settings/settings.api.ts
features/statistics/statistics.api.ts
```

The API wrappers preserve existing backend paths. They do not invent REST-renamed routes.

## State Management

Use Pinia stores for cross-page state only:

- `authStore`: current user, initialization state, login/logout actions
- `clusterStore`: selected cluster and available clusters
- `levelStore`: level list cache and refresh action
- `themeStore`: light/dark and primary color settings
- `appStore`: layout collapse state, language, global loading flags

Keep page-local form data in page components or feature components. Do not put every form field in Pinia.

## Core Page Skeletons

The first increment should implement these route-level skeletons:

- Login: calls `/api/login`, supports remember-style browser autofill, handles 401/500 messages.
- Init: calls `/api/init`, posts initial credentials when required.
- Admin layout: sidebar menu, top bar, user menu, language switch, theme switch, route guard.
- Panel: summary cards, world status table shell, start/stop action hooks wired to API wrappers where low-risk.
- Worlds: level table/list skeleton, route tabs for cluster ini, admin list, whitelist, blacklist, world settings.
- Mods: installed/subscription/manual mod page skeleton with table shell and API wrapper calls.
- Backups: backup list skeleton plus create/upload/download/restore controls that are either wired to confirmed request shapes or explicitly disabled with visible reasons.
- Settings: DST config, scheduled task, auto-check, theme setting tabs as skeletons.
- Player log, lobby, help, and user profile: navigable route pages with core API wrappers where straightforward.

Skeleton pages should render real layout, loading states, empty states, and error states. Detailed editors, mod configuration parsing, map preview, log streaming, and player operations are scoped to subsequent increments.

## Internationalization

Use `vue-i18n` with `zh`, `en`, `jp`, and `kr` resource files because the current bundle includes those languages. The first increment should include route/menu and common action labels in all four languages where existing strings are visible in the bundle. Page-specific deep strings can fall back to Chinese in the first increment, but keys must be structured so they can be completed in subsequent translation increments.

## Styling

Use Element Plus theme variables and a small project stylesheet:

- Avoid a single-hue decorative theme.
- Keep the UI dense, quiet, and operational.
- Use cards only for repeated items, summaries, and tool panels.
- Preserve usable mobile behavior for the admin shell, but optimize for desktop operations first.
- Use existing DST imagery as functional context, not as large marketing decoration.

The first increment should provide light/dark theme support matching the existing bundle's `light-bg.png` and `dark-bg.png` usage where appropriate.

## Build Integration

`web-ui/vite.config.ts` should set:

- `build.outDir = '../dist'`
- `build.emptyOutDir = true`
- `publicDir = 'public'`
- `server.proxy` for Rust API routes
- `resolve.alias` with `@` pointing to `web-ui/src`

Add root-level documentation for common commands:

```bash
cd web-ui
npm install
npm run dev
npm run type-check
npm run test:unit
npm run build
```

Do not make the Rust build depend on Node in this phase. Rust continues to serve whatever is present in `dist`.

## Testing Strategy

Add focused tests for frontend infrastructure first:

- API client normalizes success and error envelopes.
- Auth store handles login success, logout, and 401 reset.
- Router guard redirects unauthenticated users to login.
- Admin layout renders the expected menu routes.
- Core skeleton pages mount without throwing.

Use Vitest and Vue Test Utils. Browser visual verification should happen after implementation using the local dev server for desktop and mobile viewport checks.

## Error Handling

All route pages must show a usable loading, empty, and error state. API wrapper errors should preserve both HTTP status and backend `msg` where available. Authentication expiry should not leave stale user data visible.

For actions with side effects, the first increment should wire buttons only when the request payload and safety behavior are clear. Otherwise render disabled controls with explicit labels until the detailed page implementation is planned.

## Migration Risk

The main risk is expectation mismatch: the current `dist` is React, while the requested implementation is Vue. The first increment mitigates this by focusing on the contract that matters for users and the Rust backend: routes, menu, auth, API calls, assets, and core page shells.

The second risk is dependency churn from using latest packages. Committing `package-lock.json`, running type checks, and keeping the first increment infrastructure-heavy reduces that risk.

The third risk is accidentally disrupting release behavior. Keeping Rust static serving unchanged and outputting the new build to the existing `dist/` directory preserves the deployment shape.

## Acceptance Criteria

- A `web-ui/` Vue 3 + TypeScript + Vite project exists and installs cleanly.
- The installed dependency versions resolve to the latest Vue/Vite line captured in this design.
- `npm run dev` starts a frontend dev server that proxies Rust API calls.
- `npm run build` emits a production app into root `dist/`.
- The Rust server can serve the generated `dist/index.html` and assets.
- Login, admin layout, menu navigation, and core page skeletons are usable.
- API wrappers exist for auth, clusters, game, levels, mods, backups, settings, and statistics.
- Type checking and unit tests run from `web-ui`.
- Existing Rust source layout and API paths remain unchanged.
