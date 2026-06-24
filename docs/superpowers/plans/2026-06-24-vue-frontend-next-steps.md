# Vue Frontend Next Steps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Continue the Vue frontend rebuild from runnable skeleton to production-usable admin workflows without changing the Rust API surface.

**Architecture:** Keep the current `web-ui/` source tree and root `dist/` build boundary. Do not redesign the project directory again unless a page becomes too large; deepen features inside `web-ui/src/features/*`, compose route pages in `web-ui/src/pages/*`, and keep shared infrastructure in `web-ui/src/shared/*`. Use Chinese-only UI text and preserve hash routing until Rust has a deliberate SPA fallback.

**Tech Stack:** Vue 3, TypeScript, Vite, Vue Router, Pinia, Axios, Element Plus, Vitest, Vue Test Utils, vue-tsc, Rust static serving tests.

---

## File Structure

The current architecture is suitable for the next phase. Keep these ownership boundaries:

```text
web-ui/src/
├── app/                  # Vue app bootstrapping, providers, router
├── layouts/              # authenticated/admin shell and menu definitions
├── pages/                # route-level page composition
├── features/             # backend API wrappers and feature-local utilities/components
└── shared/               # API client, stores, base components, config, styles, domain types
```

Create focused feature components only when a route page starts mixing table, dialog, form, and API orchestration in one large file:

```text
web-ui/src/features/panel/
web-ui/src/features/worlds/
web-ui/src/features/mods/
web-ui/src/features/backups/
web-ui/src/features/settings/
web-ui/src/features/logs/
```

Do not introduce `i18n`, locale folders, language switches, or locale state. All display text remains direct Chinese copy or Chinese constants in `web-ui/src/shared/config/text.ts`.

## Task 1: Merge Readiness And Visual Baseline

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-06-23-vue-frontend-rebuild.md`
- Test: `web-ui/src/test/page-mount.test.ts`

- [ ] **Step 1: Record the current frontend branch state**

Run:

```bash
git status --short
git log --oneline -12
```

Expected: status is clean on `vue-frontend-rebuild`; recent commits include `35ef1ef fix: polish vue frontend shell`.

- [ ] **Step 2: Add a README frontend verification section**

Add this section to `README.md`:

````markdown
## 前端开发

前端源码位于 `web-ui/`，使用 Vue 3、TypeScript、Vite、Pinia、Vue Router 和 Element Plus。

常用命令：

```bash
cd web-ui
npm install
npm run dev
npm run test:unit -- --run
npm run build
```

生产构建输出到仓库根目录 `dist/`，由 Rust 服务继续按现有静态资源规则提供访问。
````

- [ ] **Step 3: Verify frontend commands still pass**

Run:

```bash
cd web-ui
npm run test:unit -- --run
npm run lint:check
npm run format:check
npm run build
```

Expected: all commands exit 0. `npm run build` may print Rolldown pure annotation and chunk-size warnings; record them as non-blocking warnings unless they become failures.

- [ ] **Step 4: Verify Rust static compatibility**

Run:

```bash
cargo test --test compat_manifest_tests --locked
```

Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add README.md docs/superpowers/plans/2026-06-23-vue-frontend-rebuild.md
git commit -m "docs: document vue frontend workflow"
```

## Task 2: API Contract Audit And Typed Coverage

**Files:**
- Create: `web-ui/src/features/contracts/api-contracts.ts`
- Create: `web-ui/src/test/api-contracts.test.ts`
- Modify: `web-ui/src/shared/api/types.ts`
- Modify: `web-ui/src/shared/types/domain.ts`
- Modify: `web-ui/src/features/auth/auth.api.ts`
- Modify: `web-ui/src/features/backups/backup.api.ts`
- Modify: `web-ui/src/features/clusters/cluster.api.ts`
- Modify: `web-ui/src/features/game/game.api.ts`
- Modify: `web-ui/src/features/levels/level.api.ts`
- Modify: `web-ui/src/features/mods/mod.api.ts`
- Modify: `web-ui/src/features/settings/settings.api.ts`
- Modify: `web-ui/src/features/statistics/statistics.api.ts`

- [ ] **Step 1: Create a route inventory test**

Create `web-ui/src/test/api-contracts.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { apiContracts } from '@/features/contracts/api-contracts'

describe('apiContracts', () => {
  it('keeps every frontend API path under the existing backend prefixes', () => {
    const paths = apiContracts.map((contract) => contract.path)

    expect(paths.length).toBeGreaterThan(20)
    expect(paths.every((path) => path.startsWith('/api/'))).toBe(true)
    expect(paths).toContain('/api/login')
    expect(paths).toContain('/api/game/8level/status')
    expect(paths).toContain('/api/cluster/level')
  })
})
```

- [ ] **Step 2: Run the failing test**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/api-contracts.test.ts
```

Expected: fails because `@/features/contracts/api-contracts` does not exist.

- [ ] **Step 3: Add the contract inventory**

Create `web-ui/src/features/contracts/api-contracts.ts`:

```ts
export type ApiMethod = 'GET' | 'POST' | 'PUT' | 'DELETE'

export interface ApiContract {
  method: ApiMethod
  path: string
  feature: 'auth' | 'backup' | 'cluster' | 'game' | 'level' | 'mod' | 'setting' | 'statistics'
}

export const apiContracts: ApiContract[] = [
  { method: 'POST', path: '/api/login', feature: 'auth' },
  { method: 'GET', path: '/api/logout', feature: 'auth' },
  { method: 'GET', path: '/api/init', feature: 'auth' },
  { method: 'GET', path: '/api/cluster', feature: 'cluster' },
  { method: 'GET', path: '/api/cluster/level', feature: 'level' },
  { method: 'POST', path: '/api/cluster/level', feature: 'level' },
  { method: 'PUT', path: '/api/cluster/level', feature: 'level' },
  { method: 'DELETE', path: '/api/cluster/level', feature: 'level' },
  { method: 'GET', path: '/api/game/8level/status', feature: 'game' },
  { method: 'GET', path: '/api/game/8level/start', feature: 'game' },
  { method: 'GET', path: '/api/game/8level/stop', feature: 'game' },
  { method: 'POST', path: '/api/game/8level/command', feature: 'game' },
  { method: 'GET', path: '/api/game/system/info', feature: 'game' },
  { method: 'GET', path: '/api/mod/search', feature: 'mod' },
  { method: 'GET', path: '/api/mod/config', feature: 'mod' },
  { method: 'POST', path: '/api/mod/config', feature: 'mod' },
  { method: 'GET', path: '/api/backup', feature: 'backup' },
  { method: 'POST', path: '/api/backup', feature: 'backup' },
  { method: 'DELETE', path: '/api/backup', feature: 'backup' },
  { method: 'GET', path: '/api/setting', feature: 'setting' },
  { method: 'POST', path: '/api/setting', feature: 'setting' },
  { method: 'GET', path: '/api/statistics', feature: 'statistics' },
]
```

- [ ] **Step 4: Run focused tests**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/api-contracts.test.ts src/test/api-http.test.ts
npm run type-check
```

Expected: both test files pass and type checking exits 0.

- [ ] **Step 5: Commit**

```bash
git add web-ui/src/features/contracts web-ui/src/test/api-contracts.test.ts web-ui/src/shared/api web-ui/src/shared/types web-ui/src/features
git commit -m "test: document frontend api contracts"
```

## Task 3: Dashboard And Panel Real Operations

**Files:**
- Create: `web-ui/src/features/panel/panel-actions.ts`
- Create: `web-ui/src/test/panel-actions.test.ts`
- Modify: `web-ui/src/pages/DashboardPage.vue`
- Modify: `web-ui/src/pages/PanelPage.vue`
- Modify: `web-ui/src/features/game/game.api.ts`
- Modify: `web-ui/src/shared/stores/levels.ts`

- [ ] **Step 1: Add action-state tests**

Create `web-ui/src/test/panel-actions.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { getPanelActionLabel, isLevelActionDisabled } from '@/features/panel/panel-actions'
import type { LevelSummary } from '@/shared/types/domain'

describe('panel actions', () => {
  const runningLevel: LevelSummary = { uuid: '1', levelName: 'Master', is_master: true, status: true }
  const stoppedLevel: LevelSummary = { uuid: '2', levelName: 'Caves', is_master: false, status: false }

  it('labels level operations in Chinese', () => {
    expect(getPanelActionLabel('start')).toBe('启动')
    expect(getPanelActionLabel('stop')).toBe('停止')
    expect(getPanelActionLabel('restart')).toBe('重启')
  })

  it('disables impossible start and stop actions based on runtime state', () => {
    expect(isLevelActionDisabled(runningLevel, 'start')).toBe(true)
    expect(isLevelActionDisabled(runningLevel, 'stop')).toBe(false)
    expect(isLevelActionDisabled(stoppedLevel, 'start')).toBe(false)
    expect(isLevelActionDisabled(stoppedLevel, 'stop')).toBe(true)
  })
})
```

- [ ] **Step 2: Run the failing test**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/panel-actions.test.ts
```

Expected: fails because `panel-actions.ts` does not exist.

- [ ] **Step 3: Implement action helpers**

Create `web-ui/src/features/panel/panel-actions.ts`:

```ts
import type { LevelSummary } from '@/shared/types/domain'

export type PanelAction = 'start' | 'stop' | 'restart'

export function getPanelActionLabel(action: PanelAction): string {
  return {
    start: '启动',
    stop: '停止',
    restart: '重启',
  }[action]
}

export function isLevelActionDisabled(level: LevelSummary, action: PanelAction): boolean {
  if (action === 'restart') {
    return false
  }

  if (action === 'start') {
    return Boolean(level.status)
  }

  return !level.status
}
```

- [ ] **Step 4: Wire `PanelPage.vue` to real game API actions**

Replace disabled operation buttons with buttons that call `startLevel`, `stopLevel`, and restart as stop-then-start through `web-ui/src/features/game/game.api.ts`. Use `ElMessage.success('操作已提交')` after a successful API response and always call `levelStore.refreshLevels(clusterStore.selectedCluster)` after a command settles.

- [ ] **Step 5: Run focused verification**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/panel-actions.test.ts src/test/page-mount.test.ts
npm run type-check
```

Expected: tests pass and type checking exits 0.

- [ ] **Step 6: Commit**

```bash
git add web-ui/src/features/panel web-ui/src/test/panel-actions.test.ts web-ui/src/pages/PanelPage.vue web-ui/src/pages/DashboardPage.vue web-ui/src/features/game web-ui/src/shared/stores/levels.ts
git commit -m "feat: wire panel runtime actions"
```

## Task 4: World Levels Editing Workflow

**Files:**
- Create: `web-ui/src/features/worlds/world-form.ts`
- Create: `web-ui/src/test/world-form.test.ts`
- Modify: `web-ui/src/pages/WorldLevelsPage.vue`
- Modify: `web-ui/src/features/levels/level.api.ts`
- Modify: `web-ui/src/shared/types/domain.ts`

- [ ] **Step 1: Add level form normalization tests**

Create `web-ui/src/test/world-form.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { createEmptyWorldForm, normalizeWorldForm } from '@/features/worlds/world-form'

describe('world form', () => {
  it('creates a Chinese default master world form', () => {
    expect(createEmptyWorldForm()).toEqual({
      levelName: 'Master',
      is_master: true,
      server_ini: '',
      leveldataoverride: '',
      modoverrides: '',
    })
  })

  it('trims submitted world names and keeps config strings intact', () => {
    expect(
      normalizeWorldForm({
        levelName: '  Caves  ',
        is_master: false,
        server_ini: '[NETWORK]\\nserver_port = 11001',
        leveldataoverride: 'return {}',
        modoverrides: 'return {}',
      }),
    ).toMatchObject({
      levelName: 'Caves',
      is_master: false,
      server_ini: '[NETWORK]\\nserver_port = 11001',
    })
  })
})
```

- [ ] **Step 2: Run the failing test**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/world-form.test.ts
```

Expected: fails because `world-form.ts` does not exist.

- [ ] **Step 3: Add the world form helper**

Create `web-ui/src/features/worlds/world-form.ts`:

```ts
import type { LevelPayload } from '@/features/levels/level.api'

export interface WorldForm {
  levelName: string
  is_master: boolean
  server_ini: string
  leveldataoverride: string
  modoverrides: string
}

export function createEmptyWorldForm(): WorldForm {
  return {
    levelName: 'Master',
    is_master: true,
    server_ini: '',
    leveldataoverride: '',
    modoverrides: '',
  }
}

export function normalizeWorldForm(form: WorldForm): LevelPayload {
  return {
    levelName: form.levelName.trim(),
    is_master: form.is_master,
    server_ini: form.server_ini,
    leveldataoverride: form.leveldataoverride,
    modoverrides: form.modoverrides,
  }
}
```

- [ ] **Step 4: Build the page interaction**

Update `WorldLevelsPage.vue` with:
- editable table rows for world metadata
- a create-world dialog using `createEmptyWorldForm`
- edit and delete buttons enabled
- confirmation before delete
- `ElMessage.success('世界配置已保存')` after save
- refresh after create, save, or delete

- [ ] **Step 5: Run focused verification**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/world-form.test.ts src/test/page-mount.test.ts
npm run type-check
```

Expected: tests pass and type checking exits 0.

- [ ] **Step 6: Commit**

```bash
git add web-ui/src/features/worlds web-ui/src/test/world-form.test.ts web-ui/src/pages/WorldLevelsPage.vue web-ui/src/features/levels web-ui/src/shared/types/domain.ts
git commit -m "feat: add world editing workflow"
```

## Task 5: Mods Search And Configuration Workflow

**Files:**
- Create: `web-ui/src/features/mods/mod-selection.ts`
- Create: `web-ui/src/test/mod-selection.test.ts`
- Modify: `web-ui/src/pages/ModPage.vue`
- Modify: `web-ui/src/features/mods/mod.api.ts`
- Modify: `web-ui/src/shared/types/domain.ts`

- [ ] **Step 1: Add mod selection tests**

Create `web-ui/src/test/mod-selection.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { formatWorkshopId, toggleModId } from '@/features/mods/mod-selection'

describe('mod selection', () => {
  it('normalizes workshop ids', () => {
    expect(formatWorkshopId(' workshop-123456 ')).toBe('123456')
    expect(formatWorkshopId('123456')).toBe('123456')
  })

  it('toggles mod ids without duplicates', () => {
    expect(toggleModId(['1', '2'], '2')).toEqual(['1'])
    expect(toggleModId(['1'], '2')).toEqual(['1', '2'])
  })
})
```

- [ ] **Step 2: Run the failing test**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/mod-selection.test.ts
```

Expected: fails because `mod-selection.ts` does not exist.

- [ ] **Step 3: Implement mod selection helpers**

Create `web-ui/src/features/mods/mod-selection.ts`:

```ts
export function formatWorkshopId(value: string): string {
  return value.trim().replace(/^workshop-/i, '')
}

export function toggleModId(selectedIds: string[], modId: string): string[] {
  return selectedIds.includes(modId)
    ? selectedIds.filter((id) => id !== modId)
    : [...selectedIds, modId]
}
```

- [ ] **Step 4: Wire `ModPage.vue` to real API flows**

Update `ModPage.vue` with:
- search input for workshop id or keyword
- result table with selected state
- current enabled mod list
- save button that calls the existing mod config API wrapper
- loading and empty states using `PageState`

- [ ] **Step 5: Run focused verification**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/mod-selection.test.ts src/test/page-mount.test.ts
npm run type-check
```

Expected: tests pass and type checking exits 0.

- [ ] **Step 6: Commit**

```bash
git add web-ui/src/features/mods web-ui/src/test/mod-selection.test.ts web-ui/src/pages/ModPage.vue web-ui/src/shared/types/domain.ts
git commit -m "feat: add mod management workflow"
```

## Task 6: Backup List, Create, Restore, Delete

**Files:**
- Create: `web-ui/src/features/backups/backup-format.ts`
- Create: `web-ui/src/test/backup-format.test.ts`
- Modify: `web-ui/src/pages/BackupPage.vue`
- Modify: `web-ui/src/features/backups/backup.api.ts`
- Modify: `web-ui/src/shared/types/domain.ts`

- [ ] **Step 1: Add backup display tests**

Create `web-ui/src/test/backup-format.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { formatBackupSize, getBackupActionLabel } from '@/features/backups/backup-format'

describe('backup formatting', () => {
  it('formats byte sizes for table display', () => {
    expect(formatBackupSize(0)).toBe('0 B')
    expect(formatBackupSize(1024)).toBe('1.0 KB')
    expect(formatBackupSize(1048576)).toBe('1.0 MB')
  })

  it('uses Chinese action labels', () => {
    expect(getBackupActionLabel('create')).toBe('创建备份')
    expect(getBackupActionLabel('restore')).toBe('恢复')
    expect(getBackupActionLabel('delete')).toBe('删除')
  })
})
```

- [ ] **Step 2: Run the failing test**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/backup-format.test.ts
```

Expected: fails because `backup-format.ts` does not exist.

- [ ] **Step 3: Implement backup formatting helpers**

Create `web-ui/src/features/backups/backup-format.ts`:

```ts
export type BackupAction = 'create' | 'restore' | 'delete'

export function formatBackupSize(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`
  }

  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`
  }

  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

export function getBackupActionLabel(action: BackupAction): string {
  return {
    create: '创建备份',
    restore: '恢复',
    delete: '删除',
  }[action]
}
```

- [ ] **Step 4: Wire `BackupPage.vue` to backup APIs**

Update `BackupPage.vue` with:
- backup table with name, size, created time, operation column
- create backup button
- restore confirmation dialog with backup name in the message
- delete confirmation dialog with backup name in the message
- refresh after create, restore, or delete

- [ ] **Step 5: Run focused verification**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/backup-format.test.ts src/test/page-mount.test.ts
npm run type-check
```

Expected: tests pass and type checking exits 0.

- [ ] **Step 6: Commit**

```bash
git add web-ui/src/features/backups web-ui/src/test/backup-format.test.ts web-ui/src/pages/BackupPage.vue web-ui/src/shared/types/domain.ts
git commit -m "feat: add backup management workflow"
```

## Task 7: Settings, User Profile, Help, Lobby

**Files:**
- Create: `web-ui/src/features/settings/settings-form.ts`
- Create: `web-ui/src/test/settings-form.test.ts`
- Modify: `web-ui/src/pages/SettingsPage.vue`
- Modify: `web-ui/src/pages/UserProfilePage.vue`
- Modify: `web-ui/src/pages/HelpPage.vue`
- Modify: `web-ui/src/pages/LobbyPage.vue`
- Modify: `web-ui/src/features/settings/settings.api.ts`

- [ ] **Step 1: Add settings normalization tests**

Create `web-ui/src/test/settings-form.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { normalizePanelSettings } from '@/features/settings/settings-form'

describe('settings form', () => {
  it('trims text fields and preserves boolean values', () => {
    expect(
      normalizePanelSettings({
        panelName: '  DST 管理面板  ',
        enableRegister: false,
        steamApiKey: '  key  ',
      }),
    ).toEqual({
      panelName: 'DST 管理面板',
      enableRegister: false,
      steamApiKey: 'key',
    })
  })
})
```

- [ ] **Step 2: Run the failing test**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/settings-form.test.ts
```

Expected: fails because `settings-form.ts` does not exist.

- [ ] **Step 3: Implement settings helper**

Create `web-ui/src/features/settings/settings-form.ts`:

```ts
export interface PanelSettingsForm {
  panelName: string
  enableRegister: boolean
  steamApiKey: string
}

export function normalizePanelSettings(form: PanelSettingsForm): PanelSettingsForm {
  return {
    panelName: form.panelName.trim(),
    enableRegister: form.enableRegister,
    steamApiKey: form.steamApiKey.trim(),
  }
}
```

- [ ] **Step 4: Complete the route pages**

Update pages as follows:
- `SettingsPage.vue`: form sections for panel identity, registration switch, Steam API key, save button
- `UserProfilePage.vue`: current account summary and password-change form
- `HelpPage.vue`: links to project docs and local help assets under `/misc`
- `LobbyPage.vue`: lobby query/filter layout using existing API wrappers or a read-only placeholder if the backend endpoint is not implemented yet

- [ ] **Step 5: Run focused verification**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/settings-form.test.ts src/test/page-mount.test.ts
npm run type-check
```

Expected: tests pass and type checking exits 0.

- [ ] **Step 6: Commit**

```bash
git add web-ui/src/features/settings web-ui/src/test/settings-form.test.ts web-ui/src/pages/SettingsPage.vue web-ui/src/pages/UserProfilePage.vue web-ui/src/pages/HelpPage.vue web-ui/src/pages/LobbyPage.vue
git commit -m "feat: complete settings and support pages"
```

## Task 8: Player Logs And Runtime Stream Integration

**Files:**
- Create: `web-ui/src/features/logs/log-stream.ts`
- Create: `web-ui/src/test/log-stream.test.ts`
- Modify: `web-ui/src/pages/PlayerLogPage.vue`
- Modify: `web-ui/src/features/game/game.api.ts`
- Modify: `web-ui/src/shared/api/http.ts`

- [ ] **Step 1: Add stream URL tests**

Create `web-ui/src/test/log-stream.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { buildLogStreamUrl } from '@/features/logs/log-stream'

describe('log stream', () => {
  it('builds same-origin websocket urls for browser usage', () => {
    expect(buildLogStreamUrl('http://127.0.0.1:5173', 'Cluster_1')).toBe(
      'ws://127.0.0.1:5173/ws/log?cluster=Cluster_1',
    )
  })

  it('uses secure websocket urls on https origins', () => {
    expect(buildLogStreamUrl('https://panel.example.com', 'Cluster 1')).toBe(
      'wss://panel.example.com/ws/log?cluster=Cluster+1',
    )
  })
})
```

- [ ] **Step 2: Run the failing test**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/log-stream.test.ts
```

Expected: fails because `log-stream.ts` does not exist.

- [ ] **Step 3: Implement stream URL helper**

Create `web-ui/src/features/logs/log-stream.ts`:

```ts
export function buildLogStreamUrl(origin: string, cluster: string): string {
  const url = new URL('/ws/log', origin)
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
  url.searchParams.set('cluster', cluster)
  return url.toString()
}
```

- [ ] **Step 4: Add player log stream UI**

Update `PlayerLogPage.vue` with:
- connect/disconnect button
- selected cluster display
- log level filter
- auto-scroll switch
- bounded log buffer of 1,000 rows
- empty state text `暂无日志数据`

- [ ] **Step 5: Run focused verification**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/log-stream.test.ts src/test/page-mount.test.ts
npm run type-check
```

Expected: tests pass and type checking exits 0.

- [ ] **Step 6: Commit**

```bash
git add web-ui/src/features/logs web-ui/src/test/log-stream.test.ts web-ui/src/pages/PlayerLogPage.vue web-ui/src/features/game web-ui/src/shared/api/http.ts
git commit -m "feat: add player log stream page"
```

## Task 9: Production Hardening And Browser Verification

**Files:**
- Modify: `web-ui/vite.config.ts`
- Modify: `web-ui/src/shared/styles/main.css`
- Modify: `web-ui/src/test/page-mount.test.ts`
- Modify: `dist/`

- [ ] **Step 1: Decide whether to split chunks**

If `npm run build` still reports the main chunk above 500 KB, add conservative manual chunks in `web-ui/vite.config.ts`:

```ts
build: {
  outDir: '../dist',
  emptyOutDir: true,
  rollupOptions: {
    output: {
      manualChunks: {
        vue: ['vue', 'vue-router', 'pinia'],
        element: ['element-plus', '@element-plus/icons-vue'],
      },
    },
  },
},
```

Expected: build still emits to `../dist`, and initial app behavior does not change.

- [ ] **Step 2: Run full frontend verification**

Run:

```bash
cd web-ui
npm run test:unit -- --run
npm run lint:check
npm run format:check
npm run build
```

Expected: all commands exit 0.

- [ ] **Step 3: Run Rust static compatibility**

Run:

```bash
cargo test --test compat_manifest_tests --locked
```

Expected: 4 tests pass.

- [ ] **Step 4: Verify served assets**

Start backend and check current dist references:

```bash
cargo run --bin dst-admin-rust
curl -I http://127.0.0.1:8082/
curl -I http://127.0.0.1:8082/assets/dst-emoji.woff2
```

Expected: both URLs return HTTP 200. Also check the JS and CSS filenames listed in `dist/index.html`.

- [ ] **Step 5: Commit**

```bash
git add web-ui/vite.config.ts web-ui/src/shared/styles/main.css web-ui/src/test/page-mount.test.ts
git add -f dist
git commit -m "chore: harden frontend production build"
```

## Recommended Execution Order

1. Task 1 gives a clean merge and verification baseline.
2. Task 2 makes backend route coverage explicit before adding more behavior.
3. Tasks 3 through 8 can be implemented one page group at a time with separate commits.
4. Task 9 should run after the core workflows are usable because it regenerates `dist` and may change chunk names.

## Verification Checklist

Before marking the next phase complete, run:

```bash
cd web-ui
npm run test:unit -- --run
npm run lint:check
npm run format:check
npm run build
cd ..
cargo test --test compat_manifest_tests --locked
git status --short
```

Expected:
- Vitest reports all test files passing.
- ESLint exits 0.
- Prettier reports all matched files use the configured style.
- Vite build exits 0 and writes root `dist/`.
- Rust compatibility tests pass.
- `git status --short` is clean after the final commit.
