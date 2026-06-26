# Vue Frontend Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the remaining Vue frontend pages that are still hidden or routed to `FeatureUnavailablePage`, then harden the frontend for release verification.

**Architecture:** Keep the current `web-ui/` source tree and root `dist/` build boundary. Continue placing route-level composition in `web-ui/src/pages/*`, backend wrappers in `web-ui/src/features/*`, shared state/types in `web-ui/src/shared/*`, and focused tests in `web-ui/src/test/*`. Reveal menu entries only after their pages call real backend APIs and have tests.

**Tech Stack:** Vue 3, TypeScript, Vite, Vue Router, Pinia, Axios, Element Plus, Vitest, Vue Test Utils, vue-tsc, Rust static serving tests.

## Global Constraints

- UI text is Chinese-only; do not introduce i18n, language switches, or locale state.
- Do not send a `Cluster` request header from the frontend unless the Rust API explicitly supports it and has tests.
- Keep hash routing until Rust intentionally adds a SPA fallback.
- Build output remains `dist/` at the repository root.
- Follow TDD: write failing tests first, verify red, implement minimal code, verify green, commit.
- Do not show unfinished routes in the menu. Direct access may route to `FeatureUnavailablePage`.

---

## File Structure

Create and modify these files during the next phase:

```text
web-ui/src/features/room/room.api.ts          # cluster.ini and player-list API wrappers
web-ui/src/features/room/player-lists.ts      # list kind metadata and payload mapping
web-ui/src/features/maps/map.api.ts           # map generation and image/session API wrappers
web-ui/src/features/maps/map-state.ts         # map URL/cache helpers
web-ui/src/pages/ClusterIniPage.vue           # /home/clusterIni
web-ui/src/pages/PlayerListPage.vue           # /home/adminlist, /home/whitelist, /home/blacklist
web-ui/src/pages/WorldModSelectionPage.vue    # /levels/selectorMod
web-ui/src/pages/PreinstallPage.vue           # /levels/preinstall
web-ui/src/pages/MapPreviewPage.vue           # /levels/genMap
web-ui/src/test/room-api.test.ts
web-ui/src/test/player-lists.test.ts
web-ui/src/test/cluster-ini-page.test.ts
web-ui/src/test/player-list-page.test.ts
web-ui/src/test/world-mod-selection-page.test.ts
web-ui/src/test/preinstall-page.test.ts
web-ui/src/test/map-api.test.ts
web-ui/src/test/map-preview-page.test.ts
web-ui/src/test/layout-menu.test.ts
web-ui/src/test/unavailable-routes.test.ts
web-ui/src/test/page-mount.test.ts
```

Update these existing files as each page becomes available:

```text
web-ui/src/app/router.ts
web-ui/src/layouts/menu.ts
web-ui/src/features/contracts/api-contracts.ts
web-ui/src/features/settings/settings.api.ts
web-ui/src/features/game/game.api.ts
web-ui/src/shared/types/domain.ts
```

## Task 1: Remaining API Contract Expansion

**Files:**
- Create: `web-ui/src/features/room/room.api.ts`
- Create: `web-ui/src/features/room/player-lists.ts`
- Create: `web-ui/src/features/maps/map.api.ts`
- Create: `web-ui/src/features/maps/map-state.ts`
- Modify: `web-ui/src/features/settings/settings.api.ts`
- Modify: `web-ui/src/features/game/game.api.ts`
- Modify: `web-ui/src/features/contracts/api-contracts.ts`
- Modify: `web-ui/src/shared/types/domain.ts`
- Test: `web-ui/src/test/room-api.test.ts`
- Test: `web-ui/src/test/player-lists.test.ts`
- Test: `web-ui/src/test/map-api.test.ts`
- Test: `web-ui/src/test/api-contracts.test.ts`
- Test: `web-ui/src/test/api-http.test.ts`

**Interfaces:**
- Produces `getClusterIni(): Promise<ApiEnvelope<ClusterIniEnvelope>>`
- Produces `saveClusterIni(payload: ClusterIniEnvelope): Promise<ApiEnvelope<ClusterIniEnvelope>>`
- Produces `getPlayerList(kind: PlayerListKind): Promise<ApiEnvelope<string[]>>`
- Produces `savePlayerList(kind: PlayerListKind, values: string[]): Promise<ApiEnvelope<null>>`
- Produces `getGameConfig(): Promise<ApiEnvelope<GameConfig>>`
- Produces `saveGameConfig(payload: GameConfig): Promise<ApiEnvelope<null>>`
- Produces `applyPreinstallTemplate(name: string): Promise<ApiEnvelope<null>>`
- Produces `generateMap(levelName: string): Promise<ApiEnvelope<null>>`
- Produces `getMapImageUrl(levelName: string, cacheKey?: string): string`
- Produces `checkWalrusHutPlains(levelName: string): Promise<ApiEnvelope<boolean>>`
- Produces `getSessionFile(levelName: string): Promise<ApiEnvelope<string>>`

- [ ] **Step 1: Write failing API wrapper tests**

Add tests that assert these concrete calls:

```ts
expect(get).toHaveBeenCalledWith('/api/game/8level/clusterIni')
expect(post).toHaveBeenCalledWith('/api/game/8level/clusterIni', payload, undefined)
expect(get).toHaveBeenCalledWith('/api/game/8level/adminilist')
expect(post).toHaveBeenCalledWith('/api/game/8level/whitelist', { whitelist: ['KU_abc'] }, undefined)
expect(get).toHaveBeenCalledWith('/api/dst/map/gen', { params: { levelName: 'Master' } })
expect(getMapImageUrl('Master', '123')).toBe('/api/dst/map/image?levelName=Master&t=123')
```

- [ ] **Step 2: Run focused tests and verify red**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/room-api.test.ts src/test/player-lists.test.ts src/test/map-api.test.ts
```

Expected: tests fail because the new feature modules do not exist.

- [ ] **Step 3: Implement the API wrappers**

Use exact backend paths from `src/web/app.rs`:

```ts
export type PlayerListKind = 'adminlist' | 'whitelist' | 'blacklist'

const playerListContracts = {
  adminlist: { path: '/api/game/8level/adminilist', bodyKey: 'adminList' },
  whitelist: { path: '/api/game/8level/whitelist', bodyKey: 'whitelist' },
  blacklist: { path: '/api/game/8level/blacklist', bodyKey: 'blacklist' },
} as const
```

For map images, return a URL string instead of fetching the blob in Axios so `<img>` can load it directly:

```ts
export function getMapImageUrl(levelName: string, cacheKey = ''): string {
  const params = new URLSearchParams({ levelName })
  if (cacheKey) params.set('t', cacheKey)
  return `/api/dst/map/image?${params.toString()}`
}
```

- [ ] **Step 4: Extend API contract inventory**

Add the remaining frontend-called paths to `apiContracts`:

```ts
{ method: 'GET', path: '/api/game/8level/clusterIni', feature: 'setting' }
{ method: 'POST', path: '/api/game/8level/clusterIni', feature: 'setting' }
{ method: 'GET', path: '/api/game/8level/adminilist', feature: 'game' }
{ method: 'POST', path: '/api/game/8level/adminilist', feature: 'game' }
{ method: 'GET', path: '/api/game/8level/whitelist', feature: 'game' }
{ method: 'POST', path: '/api/game/8level/whitelist', feature: 'game' }
{ method: 'GET', path: '/api/game/8level/blacklist', feature: 'game' }
{ method: 'POST', path: '/api/game/8level/blacklist', feature: 'game' }
{ method: 'GET', path: '/api/game/config', feature: 'setting' }
{ method: 'POST', path: '/api/game/config', feature: 'setting' }
{ method: 'GET', path: '/api/game/preinstall', feature: 'game' }
{ method: 'GET', path: '/api/dst/map/gen', feature: 'game' }
{ method: 'GET', path: '/api/dst/map/image', feature: 'game' }
{ method: 'GET', path: '/api/dst/map/has/walrusHut/plains', feature: 'game' }
{ method: 'GET', path: '/api/dst/map/session/file', feature: 'game' }
```

- [ ] **Step 5: Verify**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/room-api.test.ts src/test/player-lists.test.ts src/test/map-api.test.ts src/test/api-contracts.test.ts src/test/api-http.test.ts
npm run type-check
```

Expected: all focused tests and type checking pass.

- [ ] **Step 6: Commit**

```bash
git add web-ui/src/features/room web-ui/src/features/maps web-ui/src/features/settings/settings.api.ts web-ui/src/features/game/game.api.ts web-ui/src/features/contracts/api-contracts.ts web-ui/src/shared/types/domain.ts web-ui/src/test
git commit -m "feat: add remaining frontend api contracts"
```

## Task 2: Cluster Settings Page

**Files:**
- Create: `web-ui/src/pages/ClusterIniPage.vue`
- Modify: `web-ui/src/app/router.ts`
- Modify: `web-ui/src/layouts/menu.ts`
- Modify: `web-ui/src/test/page-mount.test.ts`
- Modify: `web-ui/src/test/layout-menu.test.ts`
- Modify: `web-ui/src/test/unavailable-routes.test.ts`
- Test: `web-ui/src/test/cluster-ini-page.test.ts`

**Interfaces:**
- Consumes `getClusterIni()`
- Consumes `saveClusterIni(payload)`
- Consumes `ClusterIniEnvelope`
- Produces a visible menu item at `routes.clusterIni`

- [ ] **Step 1: Write failing page tests**

Test these behaviors:

```ts
expect(getClusterIni).toHaveBeenCalled()
expect(wrapper.text()).toContain('集群设置')
expect(wrapper.text()).toContain('世界名称')
expect(saveClusterIni).toHaveBeenCalledWith({
  cluster: expect.objectContaining({
    cluster_name: '测试世界',
    max_players: 12,
    pvp: false,
  }),
  token: 'server-token',
})
```

- [ ] **Step 2: Run tests and verify red**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/cluster-ini-page.test.ts
```

Expected: fails because `ClusterIniPage.vue` is missing.

- [ ] **Step 3: Implement `ClusterIniPage.vue`**

Build an Element Plus form with these fields:

```text
世界名称 -> cluster.cluster_name
世界描述 -> cluster.cluster_description
游戏模式 -> cluster.game_mode
最大人数 -> cluster.max_players
是否 PVP -> cluster.pvp
无人暂停 -> cluster.pause_when_nobody
投票 -> cluster.vote_enabled
集群密码 -> cluster.cluster_password
令牌 -> token
主节点端口 -> cluster.master_port
```

Use numeric inputs for `max_players`, `max_snapshots`, `tick_rate`, `master_port`; use switches for booleans.

- [ ] **Step 4: Reveal route and menu**

In `router.ts`, route `routes.clusterIni` to `ClusterIniPage`. In `menu.ts`, add a `房间` submenu with only `集群设置` visible after this task.

- [ ] **Step 5: Verify**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/cluster-ini-page.test.ts src/test/layout-menu.test.ts src/test/unavailable-routes.test.ts src/test/page-mount.test.ts
npm run type-check
```

Expected: cluster settings page mounts, route is no longer unavailable, and menu shows `集群设置`.

- [ ] **Step 6: Commit**

```bash
git add web-ui/src/pages/ClusterIniPage.vue web-ui/src/app/router.ts web-ui/src/layouts/menu.ts web-ui/src/test
git commit -m "feat: add cluster settings page"
```

## Task 3: Admin, White, And Black List Pages

**Files:**
- Create: `web-ui/src/pages/PlayerListPage.vue`
- Modify: `web-ui/src/app/router.ts`
- Modify: `web-ui/src/layouts/menu.ts`
- Modify: `web-ui/src/test/page-mount.test.ts`
- Modify: `web-ui/src/test/layout-menu.test.ts`
- Modify: `web-ui/src/test/unavailable-routes.test.ts`
- Test: `web-ui/src/test/player-list-page.test.ts`

**Interfaces:**
- Consumes `getPlayerList(kind)`
- Consumes `savePlayerList(kind, values)`
- Consumes `PlayerListKind`
- Produces visible menu items for `routes.adminlist`, `routes.whitelist`, `routes.blacklist`

- [ ] **Step 1: Write failing tests for all list kinds**

Test one shared component through route props:

```ts
expect(getPlayerList).toHaveBeenCalledWith('adminlist')
expect(wrapper.text()).toContain('管理员列表')
expect(savePlayerList).toHaveBeenCalledWith('adminlist', ['KU_admin'])
expect(savePlayerList).toHaveBeenCalledWith('whitelist', ['KU_friend'])
expect(savePlayerList).toHaveBeenCalledWith('blacklist', ['KU_blocked'])
```

- [ ] **Step 2: Run tests and verify red**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/player-list-page.test.ts
```

Expected: fails because `PlayerListPage.vue` is missing.

- [ ] **Step 3: Implement `PlayerListPage.vue`**

Use one page component with props:

```ts
interface PlayerListPageProps {
  kind: PlayerListKind
  title: string
  description: string
}
```

UI behavior:

```text
加载列表 -> textarea shows one KU id per line
添加 -> appends trimmed KU id if not already present
删除 -> removes selected row
保存 -> sends deduped, non-empty lines to savePlayerList(kind, values)
```

- [ ] **Step 4: Wire routes and menu**

Map routes:

```ts
routes.adminlist -> PlayerListPage with kind adminlist and title 管理员列表
routes.whitelist -> PlayerListPage with kind whitelist and title 白名单
routes.blacklist -> PlayerListPage with kind blacklist and title 黑名单
```

Add these three entries under the `房间` submenu.

- [ ] **Step 5: Verify**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/player-list-page.test.ts src/test/layout-menu.test.ts src/test/unavailable-routes.test.ts src/test/page-mount.test.ts
npm run type-check
```

Expected: all list routes are available and no longer point to `FeatureUnavailablePage`.

- [ ] **Step 6: Commit**

```bash
git add web-ui/src/pages/PlayerListPage.vue web-ui/src/app/router.ts web-ui/src/layouts/menu.ts web-ui/src/test
git commit -m "feat: add player access list pages"
```

## Task 4: World Mod Selection Page

**Files:**
- Create: `web-ui/src/pages/WorldModSelectionPage.vue`
- Modify: `web-ui/src/features/settings/settings.api.ts`
- Modify: `web-ui/src/app/router.ts`
- Modify: `web-ui/src/layouts/menu.ts`
- Modify: `web-ui/src/test/page-mount.test.ts`
- Modify: `web-ui/src/test/layout-menu.test.ts`
- Modify: `web-ui/src/test/unavailable-routes.test.ts`
- Test: `web-ui/src/test/world-mod-selection-page.test.ts`

**Interfaces:**
- Consumes `listMods()`
- Consumes `getGameConfig()`
- Consumes `saveGameConfig(payload)`
- Produces visible menu item `routes.selectorMod`

- [ ] **Step 1: Write failing tests**

Test:

```ts
expect(listMods).toHaveBeenCalled()
expect(getGameConfig).toHaveBeenCalled()
expect(wrapper.text()).toContain('选择模组')
expect(saveGameConfig).toHaveBeenCalledWith(expect.objectContaining({
  modData: expect.stringContaining('workshop-123')
}))
```

- [ ] **Step 2: Run tests and verify red**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/world-mod-selection-page.test.ts
```

Expected: fails because the page does not exist.

- [ ] **Step 3: Implement page behavior**

Use installed mod rows from `listMods()` and current `modData` from `getGameConfig()`.

UI:

```text
左侧: installed mods table with search filter
右侧: selected mods list
底部: modoverrides.lua preview textarea
保存: writes updated GameConfig with modData
```

Use a conservative renderer:

```lua
return {
  ["workshop-123"] = { enabled = true },
}
```

- [ ] **Step 4: Reveal route and menu**

Route `routes.selectorMod` to `WorldModSelectionPage` and add `选择模组` under the `世界` submenu.

- [ ] **Step 5: Verify**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/world-mod-selection-page.test.ts src/test/layout-menu.test.ts src/test/unavailable-routes.test.ts src/test/page-mount.test.ts
npm run type-check
```

Expected: mod selection route is available and saves through `/api/game/config`.

- [ ] **Step 6: Commit**

```bash
git add web-ui/src/pages/WorldModSelectionPage.vue web-ui/src/features/settings/settings.api.ts web-ui/src/app/router.ts web-ui/src/layouts/menu.ts web-ui/src/test
git commit -m "feat: add world mod selection page"
```

## Task 5: Preinstall Template Page

**Files:**
- Create: `web-ui/src/pages/PreinstallPage.vue`
- Modify: `web-ui/src/features/game/game.api.ts`
- Modify: `web-ui/src/app/router.ts`
- Modify: `web-ui/src/layouts/menu.ts`
- Modify: `web-ui/src/test/page-mount.test.ts`
- Modify: `web-ui/src/test/layout-menu.test.ts`
- Modify: `web-ui/src/test/unavailable-routes.test.ts`
- Test: `web-ui/src/test/preinstall-page.test.ts`

**Interfaces:**
- Consumes `applyPreinstallTemplate(name)`
- Produces visible menu item `routes.preinstall`

- [ ] **Step 1: Write failing tests**

Test:

```ts
expect(wrapper.text()).toContain('预设模板')
expect(applyPreinstallTemplate).toHaveBeenCalledWith('default')
expect(wrapper.text()).toContain('会停止服务器并创建备份')
```

- [ ] **Step 2: Run tests and verify red**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/preinstall-page.test.ts
```

Expected: fails because `PreinstallPage.vue` is missing.

- [ ] **Step 3: Implement guarded operation UI**

Use an input for template name with default `default`. Require Element Plus confirmation before submit:

```text
确认文案: 应用预设会停止服务器、保存世界、创建备份并替换当前集群文件。确定继续？
```

Call:

```ts
applyPreinstallTemplate(templateName.trim() || 'default')
```

- [ ] **Step 4: Reveal route and menu**

Route `routes.preinstall` to `PreinstallPage` and add `预设模板` under the `世界` submenu.

- [ ] **Step 5: Verify**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/preinstall-page.test.ts src/test/layout-menu.test.ts src/test/unavailable-routes.test.ts src/test/page-mount.test.ts
npm run type-check
```

Expected: preinstall route is available and guarded by confirmation.

- [ ] **Step 6: Commit**

```bash
git add web-ui/src/pages/PreinstallPage.vue web-ui/src/features/game/game.api.ts web-ui/src/app/router.ts web-ui/src/layouts/menu.ts web-ui/src/test
git commit -m "feat: add preinstall template page"
```

## Task 6: Map Preview Page

**Files:**
- Create: `web-ui/src/pages/MapPreviewPage.vue`
- Modify: `web-ui/src/app/router.ts`
- Modify: `web-ui/src/layouts/menu.ts`
- Modify: `web-ui/src/test/page-mount.test.ts`
- Modify: `web-ui/src/test/layout-menu.test.ts`
- Modify: `web-ui/src/test/unavailable-routes.test.ts`
- Test: `web-ui/src/test/map-preview-page.test.ts`

**Interfaces:**
- Consumes `listLevels()`
- Consumes `generateMap(levelName)`
- Consumes `getMapImageUrl(levelName, cacheKey)`
- Consumes `checkWalrusHutPlains(levelName)`
- Consumes `getSessionFile(levelName)`
- Produces visible menu item `routes.genMap`

- [ ] **Step 1: Write failing tests**

Test:

```ts
expect(listLevels).toHaveBeenCalled()
expect(generateMap).toHaveBeenCalledWith('Master')
expect(wrapper.find('img').attributes('src')).toContain('/api/dst/map/image?levelName=Master')
expect(checkWalrusHutPlains).toHaveBeenCalledWith('Master')
expect(getSessionFile).toHaveBeenCalledWith('Master')
```

- [ ] **Step 2: Run tests and verify red**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/map-preview-page.test.ts
```

Expected: fails because `MapPreviewPage.vue` is missing.

- [ ] **Step 3: Implement map preview workflow**

UI:

```text
世界选择: Master/Caves or values from listLevels()
生成地图: calls generateMap(levelName)
地图图片: <img :src="mapImageUrl">
状态: walrus hut plains check result
会话文件: collapsible readonly textarea
```

After `generateMap`, update `cacheKey` with `Date.now().toString()` so the image reloads.

- [ ] **Step 4: Reveal route and menu**

Route `routes.genMap` to `MapPreviewPage` and add `地图预览` under the `世界` submenu.

- [ ] **Step 5: Verify**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/map-preview-page.test.ts src/test/layout-menu.test.ts src/test/unavailable-routes.test.ts src/test/page-mount.test.ts
npm run type-check
```

Expected: map preview route is available and calls real backend map APIs.

- [ ] **Step 6: Commit**

```bash
git add web-ui/src/pages/MapPreviewPage.vue web-ui/src/app/router.ts web-ui/src/layouts/menu.ts web-ui/src/test
git commit -m "feat: add map preview page"
```

## Task 7: Backup And Mod Completion Pass

**Files:**
- Modify: `web-ui/src/pages/BackupPage.vue`
- Modify: `web-ui/src/features/backups/backup.api.ts`
- Modify: `web-ui/src/pages/ModPage.vue`
- Modify: `web-ui/src/features/mods/mod.api.ts`
- Test: `web-ui/src/test/backup-api.test.ts`
- Test: `web-ui/src/test/backup-page.test.ts`
- Test: `web-ui/src/test/mod-page.test.ts`

**Interfaces:**
- Consumes existing backup routes:
  - `GET /api/game/backup/download`
  - `POST /api/game/backup/upload`
  - `PUT /api/game/backup`
- Consumes existing mod routes:
  - `POST /api/file/ugc/upload`
  - `DELETE /api/mod/setup/workshop`
  - `GET /api/mod/ugc/acf`
  - `DELETE /api/mod/ugc`

- [ ] **Step 1: Write failing tests for backup rename, download, and upload**

Test:

```ts
expect(renameBackup).toHaveBeenCalledWith({ fileName: 'old.zip', newName: 'new.zip' })
expect(downloadBackup).toHaveBeenCalledWith('backup.zip')
expect(uploadBackup).toHaveBeenCalledWith(expect.any(File))
```

- [ ] **Step 2: Implement backup completion**

Add row actions:

```text
重命名 -> opens dialog -> PUT /api/game/backup
下载 -> opens blob download
上传备份 -> file picker -> POST /api/game/backup/upload
```

- [ ] **Step 3: Write failing tests for UGC upload and cleanup**

Test:

```ts
expect(uploadUgcMod).toHaveBeenCalledWith(expect.any(FormData))
expect(deleteSetupWorkshop).toHaveBeenCalled()
expect(readUgcAcf).toHaveBeenCalled()
expect(deleteUgcMod).toHaveBeenCalledWith('workshop-123')
```

- [ ] **Step 4: Implement mod completion**

Add a restrained tools area in `ModPage.vue`:

```text
上传本地 UGC
读取 UGC ACF
清理 setup/workshop
删除本地 UGC
```

- [ ] **Step 5: Verify**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/backup-api.test.ts src/test/backup-page.test.ts src/test/mod-page.test.ts
npm run type-check
```

Expected: backup and mod completion tests pass.

- [ ] **Step 6: Commit**

```bash
git add web-ui/src/pages/BackupPage.vue web-ui/src/features/backups/backup.api.ts web-ui/src/pages/ModPage.vue web-ui/src/features/mods/mod.api.ts web-ui/src/test
git commit -m "feat: complete backup and mod tools"
```

## Task 8: UX Polish And Browser Smoke Verification

**Files:**
- Modify: `web-ui/src/shared/styles/main.css`
- Modify: `web-ui/src/layouts/AdminLayout.vue`
- Modify: `web-ui/src/pages/*.vue`
- Modify: `dist/`
- Test: `web-ui/src/test/page-mount.test.ts`
- Test: `web-ui/src/test/layout-menu.test.ts`

**Interfaces:**
- Consumes all completed routes and pages.
- Produces updated production `dist/`.

- [ ] **Step 1: Add route mount coverage for all final pages**

Ensure `page-mount.test.ts` imports every route page:

```ts
ClusterIniPage
PlayerListPage
WorldModSelectionPage
PreinstallPage
MapPreviewPage
```

- [ ] **Step 2: Verify navigation expectations**

Update `layout-menu.test.ts` so final menu contains:

```text
仪表盘
面板
房间 -> 集群设置, 管理员列表, 白名单, 黑名单
世界 -> 世界, 选择模组, 预设模板, 地图预览
模组
备份
玩家日志
设置
大厅
帮助
```

- [ ] **Step 3: Run full frontend verification**

Run:

```bash
cd web-ui
npm run test:unit -- --run
npm run lint:check
npm run format:check
npm run build
```

Expected: all commands exit 0. `npm run build` may still print the known `@vueuse/core` Rolldown annotation warning; it must not print chunk-size warnings.

- [ ] **Step 4: Run Rust static compatibility**

Run:

```bash
cargo test --locked --test http_tests static_
cargo test --test compat_manifest_tests --locked
```

Expected: static tests pass and compatibility manifest still matches implemented route surface.

- [ ] **Step 5: Browser smoke checklist**

Start backend:

```bash
cargo run --bin dst-admin-rust
```

Verify in browser:

```text
/#/init works on first-run fixtures
/#/login redirects correctly after init
/#/panel loads runtime world status
/#/home/clusterIni loads and saves form
/#/home/adminlist loads and saves KU ids
/#/home/whitelist loads and saves KU ids
/#/home/blacklist loads and saves KU ids
/#/levels/levels create/edit/delete world still works
/#/levels/selectorMod loads installed mods and saves modData
/#/levels/preinstall requires confirmation before applying
/#/levels/genMap can call generate and render image path
/#/mod, /#/backup, /#/setting, /#/playerLog still work
```

- [ ] **Step 6: Verify served production assets**

Check:

```bash
curl -I http://127.0.0.1:8082/
curl -I http://127.0.0.1:8082/assets/dst-emoji.woff2
```

Expected:

```text
/ -> 200 OK, cache-control: no-cache
/assets/dst-emoji.woff2 -> 200 OK, cache-control: public, max-age=30672000
```

Also run a local `dist/index.html` asset reference check:

```bash
node -e "const fs=require('fs'); const html=fs.readFileSync('dist/index.html','utf8'); const refs=[...html.matchAll(/(?:src|href)=\\\"(\\/assets\\/[^\\\"]+)\\\"/g)].map(m=>m[1]); const missing=refs.filter(ref=>!fs.existsSync('dist'+ref)); console.log({refs:refs.length, missing}); if(missing.length) process.exit(1)"
```

Expected: `missing` is empty.

- [ ] **Step 7: Commit**

```bash
git add web-ui/src web-ui/index.html
git add -f dist
git commit -m "chore: polish completed vue frontend"
```

## Recommended Execution Order

1. Task 1 first, because every remaining page should depend on typed API contracts rather than ad hoc HTTP calls.
2. Tasks 2 and 3 restore the hidden `房间` section.
3. Tasks 4 through 6 restore the hidden `世界` section.
4. Task 7 completes secondary tools on already-visible pages.
5. Task 8 is last because it regenerates `dist` and validates the complete browser surface.

## Final Verification Checklist

Before declaring the frontend complete, run:

```bash
cd web-ui
npm run test:unit -- --run
npm run lint:check
npm run format:check
npm run build
cd ..
cargo test --locked --test http_tests static_
cargo test --test compat_manifest_tests --locked
git status --short
```

Expected:

- Vitest reports all test files passing.
- ESLint exits 0.
- Prettier reports all matched files use the configured style.
- Vite build exits 0 and writes root `dist/`.
- Rust static and compatibility tests pass.
- `git status --short` is clean after the final commit.

## Known Non-Goals For This Plan

- Do not redesign the project directory structure.
- Do not implement backend support for arbitrary frontend-selected clusters.
- Do not add i18n.
- Do not replace Element Plus.
- Do not hand-roll map generation or DST parsing logic in the frontend.
