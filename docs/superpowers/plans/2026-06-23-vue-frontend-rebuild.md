# Vue Frontend Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a runnable Vue 3 + TypeScript + Vite frontend source project in `web-ui/` that preserves the Rust backend static serving contract and provides the API layer, admin shell, menu, and core page skeletons.

**Architecture:** Add `web-ui/` as a separate frontend source tree that builds to the existing root `dist/` directory. Keep Rust source and route behavior unchanged; the Vue app consumes existing `/api/*` endpoints through typed feature API wrappers and uses hash routing for safe refresh behavior under the current Rust static handlers. The frontend is Chinese-only: do not add `vue-i18n`, locale resource files, language switches, or locale state.

**Tech Stack:** Vue 3, TypeScript, Vite, Vue Router, Pinia, Axios, Element Plus, Monaco Editor integration points, Vitest, Vue Test Utils, vue-tsc.

**Execution status (2026-06-25):** The first frontend rebuild increment has been executed through `35ef1ef fix: polish vue frontend shell`. Follow-up implementation work is tracked in `docs/superpowers/plans/2026-06-24-vue-frontend-next-steps.md`.

---

## File Structure

Create or modify these files:

```text
web-ui/
├── index.html
├── package.json
├── package-lock.json
├── tsconfig.app.json
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
├── vitest.config.ts
├── public/
│   ├── favicon.ico
│   ├── Dst Emoji.woff2
│   ├── misc/
│   └── assets/
│       ├── dark-bg.png
│       ├── fish.gif
│       ├── light-bg.png
│       ├── login.png
│       ├── pig.gif
│       └── dst/
└── src/
    ├── app/
    │   ├── App.vue
    │   ├── main.ts
    │   ├── providers.ts
    │   └── router.ts
    ├── layouts/
    │   ├── AdminLayout.vue
    │   ├── AuthLayout.vue
    │   └── menu.ts
    ├── pages/
    │   ├── BackupPage.vue
    │   ├── DashboardPage.vue
    │   ├── HelpPage.vue
    │   ├── InitPage.vue
    │   ├── LobbyPage.vue
    │   ├── LoginPage.vue
    │   ├── ModPage.vue
    │   ├── PanelPage.vue
    │   ├── PlayerLogPage.vue
    │   ├── SettingsPage.vue
    │   ├── UserProfilePage.vue
    │   └── WorldLevelsPage.vue
    ├── features/
    │   ├── auth/auth.api.ts
    │   ├── backups/backup.api.ts
    │   ├── clusters/cluster.api.ts
    │   ├── game/game.api.ts
    │   ├── levels/level.api.ts
    │   ├── mods/mod.api.ts
    │   ├── settings/settings.api.ts
    │   └── statistics/statistics.api.ts
    ├── shared/
    │   ├── api/http.ts
    │   ├── api/types.ts
    │   ├── components/PageState.vue
    │   ├── config/routes.ts
    │   ├── config/text.ts
    │   ├── stores/app.ts
    │   ├── stores/auth.ts
    │   ├── stores/cluster.ts
    │   ├── stores/levels.ts
    │   ├── stores/theme.ts
    │   ├── styles/main.css
    │   └── types/domain.ts
    └── test/
        ├── api-http.test.ts
        ├── auth-store.test.ts
        ├── layout-menu.test.ts
        ├── page-mount.test.ts
        └── router-guard.test.ts
README.md
```

`web-ui/src/pages` owns route-level pages. `web-ui/src/features` owns API wrappers for backend route groups. `web-ui/src/shared` owns reusable infrastructure. `README.md` gets a short frontend development section.

### Task 1: Scaffold Vue/Vite Project And Build Boundary

**Files:**
- Create: `web-ui/package.json`
- Create: `web-ui/package-lock.json`
- Create: `web-ui/index.html`
- Create: `web-ui/src/app/App.vue`
- Create: `web-ui/src/app/main.ts`
- Create: `web-ui/vite.config.ts`
- Create: `web-ui/vitest.config.ts`
- Modify: `README.md`

- [ ] **Step 0: Preserve current packaged frontend assets before the first Vite build**

Before running any command that can write to the root `dist/` directory, copy the existing packaged assets to a temporary backup:

```bash
mkdir -p /private/tmp/dst-panel-original-dist-for-vue-rebuild
cp -R dist/favicon.ico dist/assets dist/misc "dist/Dst Emoji.woff2" /private/tmp/dst-panel-original-dist-for-vue-rebuild/
```

Expected: `/private/tmp/dst-panel-original-dist-for-vue-rebuild/` contains the existing favicon, font, image assets, and `misc/` files. Later tasks may restore these into `web-ui/public/` even after `npm run build` replaces root `dist/`.

- [ ] **Step 1: Create the scaffold with the official Vue CLI wrapper**

Run:

```bash
npm create vue@latest web-ui
```

When prompted, choose:

```text
Project name: web-ui
TypeScript: Yes
JSX Support: No
Vue Router: Yes
Pinia: Yes
Vitest: Yes
End-to-End Testing: No
ESLint: Yes
Prettier: Yes
```

Expected: a `web-ui/` directory exists with a Vue 3 Vite application.

- [ ] **Step 2: Install runtime and test dependencies on the latest line**

Run:

```bash
cd web-ui
npm install vue@latest vite@latest @vitejs/plugin-vue@latest vue-router@latest pinia@latest axios@latest element-plus@latest @element-plus/icons-vue@latest monaco-editor@latest @monaco-editor/loader@latest
npm install -D vue-tsc@latest vitest@latest @vue/test-utils@latest jsdom@latest
```

Expected: `package-lock.json` records concrete versions. `npm ls vue vite @vitejs/plugin-vue` reports the installed versions and no missing peer dependency errors.

- [ ] **Step 3: Replace `web-ui/vite.config.ts`**

```ts
import { fileURLToPath, URL } from 'node:url'

import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  publicDir: 'public',
  build: {
    outDir: '../dist',
    emptyOutDir: true,
  },
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://127.0.0.1:8082',
      '/ws': {
        target: 'ws://127.0.0.1:8082',
        ws: true,
      },
      '/steam': 'http://127.0.0.1:8082',
      '/webhook': 'http://127.0.0.1:8082',
      '/share': 'http://127.0.0.1:8082',
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: [],
  },
})
```

- [ ] **Step 4: Replace `web-ui/vitest.config.ts` if the scaffold creates it**

```ts
import { mergeConfig } from 'vite'
import { defineConfig } from 'vitest/config'

import viteConfig from './vite.config'

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: 'jsdom',
      globals: true,
    },
  }),
)
```

- [ ] **Step 5: Normalize `web-ui/package.json` scripts**

Ensure these scripts exist:

```json
{
  "scripts": {
    "dev": "vite",
    "build": "npm run type-check && npm run build-only",
    "preview": "vite preview",
    "test:unit": "vitest",
    "build-only": "vite build",
    "type-check": "vue-tsc --build",
    "lint": "eslint . --fix",
    "format": "prettier --write src/",
    "format:check": "prettier --check src/",
    "lint:check": "eslint ."
  }
}
```

Expected: `npm run build` runs type-check before `vite build`; do not use `run-p` here because production builds write to root `dist/`.

- [ ] **Step 6: Replace `web-ui/index.html`**

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" href="/favicon.ico" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>DST Admin</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/app/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 7: Run baseline commands**

Run:

```bash
cd web-ui
npm run type-check
npm run test:unit -- --run
npm run build
```

Expected: type-check, unit tests, and build pass. The root `dist/index.html` references Vite-generated `/assets/*` chunks.

- [ ] **Step 8: Add README frontend commands**

Append this section to `README.md`:

```md
## 前端开发

前端源码位于 `web-ui/`，使用 Vue 3、TypeScript 和 Vite。生产构建输出到根目录 `dist/`，由 Rust 服务端继续按现有静态文件规则提供访问。

```bash
cd web-ui
npm install
npm run dev
npm run type-check
npm run test:unit -- --run
npm run build
```
```

- [ ] **Step 9: Commit**

```bash
git add README.md web-ui
git commit -m "feat: scaffold vue frontend"
```

### Task 2: Shared API Client And Feature API Wrappers

**Files:**
- Create: `web-ui/src/shared/api/types.ts`
- Create: `web-ui/src/shared/api/http.ts`
- Create: `web-ui/src/features/auth/auth.api.ts`
- Create: `web-ui/src/features/backups/backup.api.ts`
- Create: `web-ui/src/features/clusters/cluster.api.ts`
- Create: `web-ui/src/features/game/game.api.ts`
- Create: `web-ui/src/features/levels/level.api.ts`
- Create: `web-ui/src/features/mods/mod.api.ts`
- Create: `web-ui/src/features/settings/settings.api.ts`
- Create: `web-ui/src/features/statistics/statistics.api.ts`
- Create: `web-ui/src/shared/types/domain.ts`
- Test: `web-ui/src/test/api-http.test.ts`

- [ ] **Step 1: Write the failing API tests**

Create `web-ui/src/test/api-http.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { isApiSuccess, normalizeApiError } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'

describe('api helpers', () => {
  it('treats code 0 and code 200 as successful legacy envelopes', () => {
    const zero: ApiEnvelope<string> = { code: 0, msg: '', data: 'ok' }
    const login: ApiEnvelope<string> = { code: 200, msg: 'success', data: 'ok' }

    expect(isApiSuccess(zero)).toBe(true)
    expect(isApiSuccess(login)).toBe(true)
  })

  it('normalizes backend messages and http status into a readable error', () => {
    const error = normalizeApiError({
      response: {
        status: 401,
        data: { code: 401, msg: 'unauthorized' },
      },
    })

    expect(error.status).toBe(401)
    expect(error.message).toBe('unauthorized')
  })
})
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/api-http.test.ts
```

Expected: FAIL because `@/shared/api/http` does not exist.

- [ ] **Step 3: Add API envelope types**

Create `web-ui/src/shared/api/types.ts`:

```ts
export interface ApiEnvelope<T = unknown> {
  code: number
  msg?: string
  data: T
}

export interface ApiError {
  status?: number
  code?: number
  message: string
  raw: unknown
}

export interface PageResult<T> {
  list: T[]
  total?: number
}
```

- [ ] **Step 4: Add domain types**

Create `web-ui/src/shared/types/domain.ts`:

```ts
export interface UserProfile {
  username: string
  displayName: string
  photoURL?: string
}

export interface LoginRequest {
  username: string
  password: string
}

export interface InitRequest extends LoginRequest {
  displayName: string
  photoURL?: string
}

export interface ClusterSummary {
  id?: number
  name?: string
  clusterName?: string
  description?: string
  createdAt?: string
  updatedAt?: string
}

export interface LevelSummary {
  uuid: string
  levelName: string
  status?: boolean
  ps?: unknown
  Ps?: unknown
  leveldataoverride?: string
}

export interface BackupFile {
  fileName: string
  fileSize?: number
  createTime?: string
}

export interface ModSummary {
  id?: number
  modid?: string
  workshopId?: string
  name?: string
  author?: string
  version?: string
}

export interface TaskSummary {
  jobId: string
  cron: string
  category: string
  levelName?: string
  valid?: boolean
}
```

- [ ] **Step 5: Add the shared Axios client**

Create `web-ui/src/shared/api/http.ts`:

```ts
import axios, { AxiosError, type AxiosRequestConfig } from 'axios'

import type { ApiEnvelope, ApiError } from './types'

export const http = axios.create({
  baseURL: '/',
  withCredentials: true,
})

export function isApiSuccess(envelope: Pick<ApiEnvelope, 'code'>): boolean {
  return envelope.code === 0 || envelope.code === 200
}

export function normalizeApiError(error: unknown): ApiError {
  if (axios.isAxiosError(error)) {
    const axiosError = error as AxiosError<{ code?: number; msg?: string }>
    const status = axiosError.response?.status
    const code = axiosError.response?.data?.code
    const message =
      axiosError.response?.data?.msg ||
      axiosError.message ||
      (status ? `HTTP ${status}` : 'Request failed')

    return { status, code, message, raw: error }
  }

  if (error instanceof Error) {
    return { message: error.message, raw: error }
  }

  return { message: 'Request failed', raw: error }
}

export async function apiGet<T>(url: string, config?: AxiosRequestConfig): Promise<ApiEnvelope<T>> {
  const response = await http.get<ApiEnvelope<T>>(url, config)
  return response.data
}

export async function apiPost<T>(
  url: string,
  data?: unknown,
  config?: AxiosRequestConfig,
): Promise<ApiEnvelope<T>> {
  const response = await http.post<ApiEnvelope<T>>(url, data, config)
  return response.data
}

export async function apiPut<T>(
  url: string,
  data?: unknown,
  config?: AxiosRequestConfig,
): Promise<ApiEnvelope<T>> {
  const response = await http.put<ApiEnvelope<T>>(url, data, config)
  return response.data
}

export async function apiDelete<T>(
  url: string,
  config?: AxiosRequestConfig,
): Promise<ApiEnvelope<T>> {
  const response = await http.delete<ApiEnvelope<T>>(url, config)
  return response.data
}

export function withCluster(cluster?: string): AxiosRequestConfig | undefined {
  return cluster ? { headers: { Cluster: cluster } } : undefined
}
```

- [ ] **Step 6: Add feature API wrappers**

Create `web-ui/src/features/auth/auth.api.ts`:

```ts
import { apiGet, apiPost } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { InitRequest, LoginRequest, UserProfile } from '@/shared/types/domain'

export function checkInit(): Promise<ApiEnvelope<boolean>> {
  return apiGet<boolean>('/api/init')
}

export function initFirstUser(payload: InitRequest): Promise<ApiEnvelope<UserProfile>> {
  return apiPost<UserProfile>('/api/init', payload)
}

export function login(payload: LoginRequest): Promise<ApiEnvelope<UserProfile>> {
  return apiPost<UserProfile>('/api/login', payload)
}

export function logout(): Promise<ApiEnvelope<unknown>> {
  return apiGet<unknown>('/api/logout')
}

export function getUser(): Promise<ApiEnvelope<UserProfile>> {
  return apiGet<UserProfile>('/api/user')
}

export function updateUser(payload: UserProfile & { password?: string }): Promise<ApiEnvelope<UserProfile>> {
  return apiPost<UserProfile>('/api/user', payload)
}
```

Create `web-ui/src/features/clusters/cluster.api.ts`:

```ts
import { apiDelete, apiGet, apiPost, apiPut } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { ClusterSummary } from '@/shared/types/domain'

export function listClusters(): Promise<ApiEnvelope<ClusterSummary[]>> {
  return apiGet<ClusterSummary[]>('/api/cluster')
}

export function createCluster(payload: ClusterSummary): Promise<ApiEnvelope<ClusterSummary>> {
  return apiPost<ClusterSummary>('/api/cluster', payload)
}

export function updateCluster(payload: ClusterSummary): Promise<ApiEnvelope<ClusterSummary>> {
  return apiPut<ClusterSummary>('/api/cluster', payload)
}

export function deleteCluster(id: number): Promise<ApiEnvelope<unknown>> {
  return apiDelete<unknown>('/api/cluster', { params: { id } })
}
```

Create `web-ui/src/features/levels/level.api.ts`:

```ts
import { apiDelete, apiGet, apiPost, apiPut, withCluster } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { LevelSummary } from '@/shared/types/domain'

export function listLevels(cluster?: string): Promise<ApiEnvelope<LevelSummary[]>> {
  return apiGet<LevelSummary[]>('/api/cluster/level', withCluster(cluster))
}

export function saveLevels(cluster: string | undefined, levels: LevelSummary[]): Promise<ApiEnvelope<LevelSummary[]>> {
  return apiPut<LevelSummary[]>('/api/cluster/level', levels, withCluster(cluster))
}

export function createLevel(cluster: string | undefined, payload: Partial<LevelSummary>): Promise<ApiEnvelope<LevelSummary>> {
  return apiPost<LevelSummary>('/api/cluster/level', payload, withCluster(cluster))
}

export function deleteLevel(cluster: string | undefined, levelName: string): Promise<ApiEnvelope<unknown>> {
  return apiDelete<unknown>('/api/cluster/level', {
    ...withCluster(cluster),
    params: { levelName },
  })
}
```

Create `web-ui/src/features/game/game.api.ts`:

```ts
import { apiGet, apiPost, withCluster } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { LevelSummary } from '@/shared/types/domain'

export function getLevelStatus(cluster?: string): Promise<ApiEnvelope<LevelSummary[]>> {
  return apiGet<LevelSummary[]>('/api/game/8level/status', withCluster(cluster))
}

export function startLevel(cluster: string | undefined, levelName: string): Promise<ApiEnvelope<unknown>> {
  return apiGet<unknown>('/api/game/8level/start', {
    ...withCluster(cluster),
    params: { levelName },
  })
}

export function stopLevel(cluster: string | undefined, levelName: string): Promise<ApiEnvelope<unknown>> {
  return apiGet<unknown>('/api/game/8level/stop', {
    ...withCluster(cluster),
    params: { levelName },
  })
}

export function sendLevelCommand(
  cluster: string | undefined,
  levelName: string,
  command: string,
): Promise<ApiEnvelope<unknown>> {
  return apiPost<unknown>('/api/game/8level/command', { levelName, command }, withCluster(cluster))
}

export function getSystemInfo(cluster?: string): Promise<ApiEnvelope<unknown>> {
  return apiGet<unknown>('/api/game/system/info', withCluster(cluster))
}
```

Create `web-ui/src/features/mods/mod.api.ts`:

```ts
import { apiDelete, apiGet, apiPost, apiPut, withCluster } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { ModSummary } from '@/shared/types/domain'

export function listMods(cluster?: string): Promise<ApiEnvelope<ModSummary[]>> {
  return apiGet<ModSummary[]>('/api/mod', withCluster(cluster))
}

export function searchMods(keyword: string): Promise<ApiEnvelope<ModSummary[]>> {
  return apiGet<ModSummary[]>('/api/mod/search', { params: { keyword } })
}

export function getMod(modId: string): Promise<ApiEnvelope<ModSummary>> {
  return apiGet<ModSummary>(`/api/mod/${encodeURIComponent(modId)}`)
}

export function updateMod(modId: string, payload: unknown): Promise<ApiEnvelope<unknown>> {
  return apiPut<unknown>(`/api/mod/${encodeURIComponent(modId)}`, payload)
}

export function deleteMod(modId: string): Promise<ApiEnvelope<unknown>> {
  return apiDelete<unknown>(`/api/mod/${encodeURIComponent(modId)}`)
}

export function saveRawModinfo(payload: unknown): Promise<ApiEnvelope<unknown>> {
  return apiPost<unknown>('/api/mod/modinfo', payload)
}
```

Create `web-ui/src/features/backups/backup.api.ts`:

```ts
import { apiDelete, apiGet, apiPost, apiPut, http, withCluster } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { BackupFile } from '@/shared/types/domain'

export function listBackups(cluster?: string): Promise<ApiEnvelope<BackupFile[]>> {
  return apiGet<BackupFile[]>('/api/game/backup', withCluster(cluster))
}

export function createBackup(cluster?: string): Promise<ApiEnvelope<BackupFile>> {
  return apiPost<BackupFile>('/api/game/backup', undefined, withCluster(cluster))
}

export function deleteBackup(cluster: string | undefined, fileName: string): Promise<ApiEnvelope<unknown>> {
  return apiDelete<unknown>('/api/game/backup', {
    ...withCluster(cluster),
    params: { fileName },
  })
}

export function renameBackup(
  cluster: string | undefined,
  fileName: string,
  newBackupName: string,
): Promise<ApiEnvelope<BackupFile>> {
  return apiPut<BackupFile>('/api/game/backup', { fileName, newBackupName }, withCluster(cluster))
}

export async function downloadBackup(cluster: string | undefined, fileName: string): Promise<Blob> {
  const response = await http.get('/api/game/backup/download', {
    ...withCluster(cluster),
    params: { fileName },
    responseType: 'blob',
  })
  return response.data
}
```

Create `web-ui/src/features/settings/settings.api.ts`:

```ts
import { apiDelete, apiGet, apiPost } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { TaskSummary } from '@/shared/types/domain'

export function getDstConfig(): Promise<ApiEnvelope<unknown>> {
  return apiGet<unknown>('/api/dst/config')
}

export function saveDstConfig(payload: unknown): Promise<ApiEnvelope<unknown>> {
  return apiPost<unknown>('/api/dst/config', payload)
}

export function listTasks(): Promise<ApiEnvelope<TaskSummary[]>> {
  return apiGet<TaskSummary[]>('/api/task')
}

export function createTask(payload: unknown): Promise<ApiEnvelope<TaskSummary>> {
  return apiPost<TaskSummary>('/api/task', payload)
}

export function deleteTask(jobId: string): Promise<ApiEnvelope<unknown>> {
  return apiDelete<unknown>('/api/task', { params: { jobId } })
}

export function getAutoCheck(checkType: string): Promise<ApiEnvelope<unknown[]>> {
  return apiGet<unknown[]>('/api/auto/check2', { params: { checkType } })
}

export function saveAutoCheck(payload: unknown): Promise<ApiEnvelope<unknown>> {
  return apiPost<unknown>('/api/auto/check2', payload)
}
```

Create `web-ui/src/features/statistics/statistics.api.ts`:

```ts
import { apiGet } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'

export function getActiveUsers(range?: string): Promise<ApiEnvelope<unknown>> {
  return apiGet<unknown>('/api/statistics/active/user', { params: { range } })
}

export function getTopActive(): Promise<ApiEnvelope<unknown>> {
  return apiGet<unknown>('/api/statistics/top/active')
}

export function getRoleRate(): Promise<ApiEnvelope<unknown>> {
  return apiGet<unknown>('/api/statistics/rate/role')
}

export function getRegenerateStats(): Promise<ApiEnvelope<unknown>> {
  return apiGet<unknown>('/api/statistics/regenerate')
}
```

- [ ] **Step 7: Run tests and type check**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/api-http.test.ts
npm run type-check
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add web-ui/src/shared/api web-ui/src/shared/types web-ui/src/features web-ui/src/test/api-http.test.ts
git commit -m "feat: add frontend api layer"
```

### Task 3: Pinia Stores And Chinese Text Constants

**Files:**
- Create: `web-ui/src/shared/config/text.ts`
- Create: `web-ui/src/shared/stores/app.ts`
- Create: `web-ui/src/shared/stores/auth.ts`
- Create: `web-ui/src/shared/stores/cluster.ts`
- Create: `web-ui/src/shared/stores/levels.ts`
- Create: `web-ui/src/shared/stores/theme.ts`
- Create: `web-ui/src/app/providers.ts`
- Test: `web-ui/src/test/auth-store.test.ts`

Language policy: this project is Chinese-only. Do not add `vue-i18n`, locale files, language switchers, `locale` store state, or `localStorage.language` handling.

- [ ] **Step 1: Write the failing auth store test**

Create `web-ui/src/test/auth-store.test.ts`:

```ts
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { useAuthStore } from '@/shared/stores/auth'

vi.mock('@/features/auth/auth.api', () => ({
  getUser: vi.fn(async () => ({ code: 200, data: { username: 'admin', displayName: '管理员' } })),
  login: vi.fn(async () => ({ code: 200, data: { username: 'admin', displayName: '管理员' } })),
  logout: vi.fn(async () => ({ code: 200, data: null })),
}))

describe('auth store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('stores the current user after login and clears it after logout', async () => {
    const store = useAuthStore()

    await store.loginWithPassword({ username: 'admin', password: 'secret' })
    expect(store.user?.displayName || store.user?.username).toBe('管理员')
    expect(store.isAuthenticated).toBe(true)

    await store.logoutUser()
    expect(store.user).toBeNull()
    expect(store.isAuthenticated).toBe(false)
  })
})
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/auth-store.test.ts
```

Expected: FAIL because `@/shared/stores/auth` does not exist.

- [ ] **Step 3: Add reusable Chinese text constants**

Create `web-ui/src/shared/config/text.ts`:

```ts
export const appText = {
  title: '饥荒管理控制台',
  common: {
    loading: '加载中',
    empty: '暂无数据',
    error: '加载失败',
    save: '保存',
    refresh: '刷新',
    disabled: '后续增量实现',
  },
  auth: {
    login: '登录',
    username: '用户名',
    password: '密码',
    logout: '退出登录',
  },
  menu: {
    dashboard: '仪表盘',
    panel: '面板',
    home: '房间',
    clusterIni: '集群设置',
    adminlist: '管理员列表',
    whitelist: '白名单',
    blacklist: '黑名单',
    levels: '世界',
    selectorMod: '选择模组',
    preinstall: '预设模板',
    genMap: '地图预览',
    mod: '模组',
    backup: '备份',
    playerLog: '玩家日志',
    setting: '设置',
    lobby: '大厅',
    help: '帮助',
    userProfile: '个人信息',
  },
} as const
```

Use these constants only where reuse helps. Page-specific labels can remain direct Chinese strings inside components.

- [ ] **Step 4: Add stores**

Create `web-ui/src/shared/stores/auth.ts`:

```ts
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

import * as authApi from '@/features/auth/auth.api'
import { isApiSuccess } from '@/shared/api/http'
import type { LoginRequest, UserProfile } from '@/shared/types/domain'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<UserProfile | null>(null)
  const loading = ref(false)
  const initialized = ref(false)

  const isAuthenticated = computed(() => user.value !== null)

  async function fetchCurrentUser(): Promise<void> {
    loading.value = true
    try {
      const response = await authApi.getUser()
      user.value = isApiSuccess(response) ? response.data : null
    } catch {
      user.value = null
    } finally {
      loading.value = false
      initialized.value = true
    }
  }

  async function loginWithPassword(payload: LoginRequest): Promise<void> {
    loading.value = true
    try {
      const response = await authApi.login(payload)
      if (!isApiSuccess(response)) {
        throw new Error(response.msg || '登录失败')
      }
      user.value = response.data
      initialized.value = true
    } finally {
      loading.value = false
    }
  }

  async function logoutUser(): Promise<void> {
    await authApi.logout()
    user.value = null
    initialized.value = false
  }

  function clearAuth(): void {
    user.value = null
    initialized.value = false
  }

  return {
    user,
    loading,
    initialized,
    isAuthenticated,
    fetchCurrentUser,
    loginWithPassword,
    logoutUser,
    clearAuth,
  }
})
```

Create `web-ui/src/shared/stores/app.ts`:

```ts
import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useAppStore = defineStore('app', () => {
  const sidebarCollapsed = ref(false)
  const globalLoading = ref(false)

  function setSidebarCollapsed(value: boolean): void {
    sidebarCollapsed.value = value
  }

  function setGlobalLoading(value: boolean): void {
    globalLoading.value = value
  }

  return { sidebarCollapsed, globalLoading, setSidebarCollapsed, setGlobalLoading }
})
```

Create `web-ui/src/shared/stores/cluster.ts`:

```ts
import { defineStore } from 'pinia'
import { ref } from 'vue'

import { listClusters } from '@/features/clusters/cluster.api'
import { isApiSuccess } from '@/shared/api/http'
import type { ClusterSummary } from '@/shared/types/domain'

export const useClusterStore = defineStore('cluster', () => {
  const selectedCluster = ref('')
  const clusters = ref<ClusterSummary[]>([])
  const loading = ref(false)

  async function refreshClusters(): Promise<void> {
    loading.value = true
    try {
      const response = await listClusters()
      const data = response.data
      clusters.value = isApiSuccess(response) ? (Array.isArray(data) ? data : data.data || []) : []
    } finally {
      loading.value = false
    }
  }

  function setSelectedCluster(value: string): void {
    selectedCluster.value = value
  }

  return { selectedCluster, clusters, loading, refreshClusters, setSelectedCluster }
})
```

Create `web-ui/src/shared/stores/levels.ts`:

```ts
import { defineStore } from 'pinia'
import { ref } from 'vue'

import { listLevels } from '@/features/levels/level.api'
import { isApiSuccess } from '@/shared/api/http'
import type { LevelSummary } from '@/shared/types/domain'

export const useLevelStore = defineStore('levels', () => {
  const levels = ref<LevelSummary[]>([])
  const loading = ref(false)

  async function refreshLevels(cluster?: string): Promise<void> {
    loading.value = true
    try {
      const response = await listLevels(cluster)
      levels.value = isApiSuccess(response) ? response.data : []
    } finally {
      loading.value = false
    }
  }

  return { levels, loading, refreshLevels }
})
```

Create `web-ui/src/shared/stores/theme.ts`:

```ts
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

export const useThemeStore = defineStore('theme', () => {
  const mode = ref<'light' | 'dark'>((localStorage.getItem('theme') as 'light' | 'dark') || 'light')
  const primaryColor = ref(localStorage.getItem('primaryColor') || '#1f7a4d')

  const isDark = computed(() => mode.value === 'dark')

  function setMode(value: 'light' | 'dark'): void {
    mode.value = value
    localStorage.setItem('theme', value)
  }

  function setPrimaryColor(value: string): void {
    primaryColor.value = value
    localStorage.setItem('primaryColor', value)
  }

  return { mode, primaryColor, isDark, setMode, setPrimaryColor }
})
```

- [ ] **Step 5: Add providers**

Create `web-ui/src/app/providers.ts`:

```ts
import ElementPlus from 'element-plus'
import { createPinia } from 'pinia'
import type { App } from 'vue'

export function installProviders(app: App): void {
  app.use(createPinia())
  app.use(ElementPlus)
}
```

- [ ] **Step 6: Run test and type check**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/auth-store.test.ts
npm run type-check
npm run lint:check
npm run format:check
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add web-ui/src/shared/config/text.ts web-ui/src/shared/stores web-ui/src/app/providers.ts web-ui/src/test/auth-store.test.ts
git commit -m "feat: add frontend app state"
```

### Task 4: Router, Menu, Layout, And Route Guard

**Files:**
- Create: `web-ui/src/shared/config/routes.ts`
- Create: `web-ui/src/layouts/menu.ts`
- Create: `web-ui/src/layouts/AuthLayout.vue`
- Create: `web-ui/src/layouts/AdminLayout.vue`
- Create: `web-ui/src/app/router.ts`
- Modify: `web-ui/src/app/App.vue`
- Modify: `web-ui/src/app/main.ts`
- Create: `web-ui/src/shared/styles/main.css`
- Test: `web-ui/src/test/layout-menu.test.ts`
- Test: `web-ui/src/test/router-guard.test.ts`

- [ ] **Step 1: Write failing menu test**

Create `web-ui/src/test/layout-menu.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { adminMenuItems } from '@/layouts/menu'

describe('admin menu', () => {
  it('contains the core operational routes', () => {
    const paths = JSON.stringify(adminMenuItems)

    expect(paths).toContain('/panel')
    expect(paths).toContain('/levels/levels')
    expect(paths).toContain('/mod')
    expect(paths).toContain('/backup')
    expect(paths).toContain('/setting')
  })
})
```

- [ ] **Step 2: Write failing router guard test**

Create `web-ui/src/test/router-guard.test.ts`:

```ts
import { createPinia, setActivePinia } from 'pinia'
import { describe, expect, it } from 'vitest'

import { createAppRouter } from '@/app/router'

describe('router guard', () => {
  it('redirects protected routes to login when no user is loaded', async () => {
    setActivePinia(createPinia())
    const router = createAppRouter()

    await router.push('/panel')
    await router.isReady()

    expect(router.currentRoute.value.path).toBe('/login')
  })
})
```

- [ ] **Step 3: Run tests and verify RED**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/layout-menu.test.ts src/test/router-guard.test.ts
```

Expected: FAIL because router and menu files do not exist.

- [ ] **Step 4: Add route constants**

Create `web-ui/src/shared/config/routes.ts`:

```ts
export const routes = {
  login: '/login',
  init: '/init',
  dashboard: '/dashboard',
  panel: '/panel',
  clusterIni: '/home/clusterIni',
  adminlist: '/home/adminlist',
  whitelist: '/home/whitelist',
  blacklist: '/home/blacklist',
  levels: '/levels/levels',
  selectorMod: '/levels/selectorMod',
  preinstall: '/levels/preinstall',
  genMap: '/levels/genMap',
  mod: '/mod',
  backup: '/backup',
  playerLog: '/playerLog',
  setting: '/setting',
  lobby: '/lobby',
  help: '/help',
  userProfile: '/userProfile',
} as const
```

- [ ] **Step 5: Add menu model**

Create `web-ui/src/layouts/menu.ts`:

```ts
import {
  Box,
  DataAnalysis,
  Document,
  Files,
  House,
  Monitor,
  Operation,
  Setting,
  UserFilled,
} from '@element-plus/icons-vue'

import { routes } from '@/shared/config/routes'

export interface AdminMenuItem {
  path: string
  label: string
  icon?: unknown
  children?: AdminMenuItem[]
}

export const adminMenuItems: AdminMenuItem[] = [
  { path: routes.dashboard, label: '仪表盘', icon: DataAnalysis },
  { path: routes.panel, label: '面板', icon: Monitor },
  {
    path: '/home',
    label: '房间',
    icon: House,
    children: [
      { path: routes.clusterIni, label: '集群设置' },
      { path: routes.adminlist, label: '管理员列表' },
      { path: routes.whitelist, label: '白名单' },
      { path: routes.blacklist, label: '黑名单' },
    ],
  },
  {
    path: '/levels',
    label: '世界',
    icon: Operation,
    children: [
      { path: routes.levels, label: '世界' },
      { path: routes.selectorMod, label: '选择模组' },
      { path: routes.preinstall, label: '预设模板' },
      { path: routes.genMap, label: '地图预览' },
    ],
  },
  { path: routes.mod, label: '模组', icon: Box },
  { path: routes.backup, label: '备份', icon: Files },
  { path: routes.playerLog, label: '玩家日志', icon: UserFilled },
  { path: routes.setting, label: '设置', icon: Setting },
  { path: routes.lobby, label: '大厅', icon: House },
  { path: routes.help, label: '帮助', icon: Document },
]
```

- [ ] **Step 6: Add layouts**

Create `web-ui/src/layouts/AuthLayout.vue`:

```vue
<template>
  <main class="auth-layout">
    <RouterView />
  </main>
</template>
```

Create `web-ui/src/layouts/AdminLayout.vue`:

```vue
<script setup lang="ts">
import { ArrowDown, Moon, Sunny } from '@element-plus/icons-vue'
import { computed } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'

import { adminMenuItems, type AdminMenuItem } from '@/layouts/menu'
import { useAuthStore } from '@/shared/stores/auth'
import { useThemeStore } from '@/shared/stores/theme'

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const theme = useThemeStore()

const activePath = computed(() => route.path)

function openMenu(item: AdminMenuItem): void {
  router.push(item.path)
}

async function logout(): Promise<void> {
  await auth.logoutUser()
  await router.replace('/login')
}
</script>

<template>
  <el-container class="admin-layout">
    <el-aside width="232px" class="admin-layout__aside">
      <div class="admin-layout__brand" @click="router.push('/panel')">DST Panel</div>
      <el-menu :default-active="activePath" router class="admin-layout__menu">
        <template v-for="item in adminMenuItems" :key="item.path">
          <el-sub-menu v-if="item.children" :index="item.path">
            <template #title>
              <el-icon v-if="item.icon"><component :is="item.icon" /></el-icon>
              <span>{{ item.label }}</span>
            </template>
            <el-menu-item
              v-for="child in item.children"
              :key="child.path"
              :index="child.path"
              @click="openMenu(child)"
            >
              {{ child.label }}
            </el-menu-item>
          </el-sub-menu>
          <el-menu-item v-else :index="item.path" @click="openMenu(item)">
            <el-icon v-if="item.icon"><component :is="item.icon" /></el-icon>
            <span>{{ item.label }}</span>
          </el-menu-item>
        </template>
      </el-menu>
    </el-aside>

    <el-container>
      <el-header class="admin-layout__header">
        <div />
        <div class="admin-layout__actions">
          <el-button
            :icon="theme.isDark ? Sunny : Moon"
            circle
            @click="theme.setMode(theme.isDark ? 'light' : 'dark')"
          />
          <el-dropdown>
            <el-button>
              {{ auth.user?.displayName || auth.user?.username || 'admin' }}
              <el-icon class="el-icon--right"><ArrowDown /></el-icon>
            </el-button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item @click="router.push('/userProfile')">个人信息</el-dropdown-item>
                <el-dropdown-item divided @click="logout">退出登录</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </el-header>

      <el-main class="admin-layout__main">
        <RouterView />
      </el-main>
    </el-container>
  </el-container>
</template>
```

- [ ] **Step 7: Add router**

Create `web-ui/src/app/router.ts`:

```ts
import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'

import AdminLayout from '@/layouts/AdminLayout.vue'
import AuthLayout from '@/layouts/AuthLayout.vue'
import BackupPage from '@/pages/BackupPage.vue'
import DashboardPage from '@/pages/DashboardPage.vue'
import HelpPage from '@/pages/HelpPage.vue'
import InitPage from '@/pages/InitPage.vue'
import LobbyPage from '@/pages/LobbyPage.vue'
import LoginPage from '@/pages/LoginPage.vue'
import ModPage from '@/pages/ModPage.vue'
import PanelPage from '@/pages/PanelPage.vue'
import PlayerLogPage from '@/pages/PlayerLogPage.vue'
import SettingsPage from '@/pages/SettingsPage.vue'
import UserProfilePage from '@/pages/UserProfilePage.vue'
import WorldLevelsPage from '@/pages/WorldLevelsPage.vue'
import { routes } from '@/shared/config/routes'
import { useAuthStore } from '@/shared/stores/auth'

const routeRecords: RouteRecordRaw[] = [
  {
    path: '/',
    redirect: routes.panel,
  },
  {
    path: '/',
    component: AuthLayout,
    children: [
      { path: routes.login, component: LoginPage, meta: { public: true } },
      { path: routes.init, component: InitPage, meta: { public: true } },
    ],
  },
  {
    path: '/',
    component: AdminLayout,
    children: [
      { path: routes.dashboard, component: DashboardPage },
      { path: routes.panel, component: PanelPage },
      { path: routes.clusterIni, component: WorldLevelsPage },
      { path: routes.adminlist, component: WorldLevelsPage },
      { path: routes.whitelist, component: WorldLevelsPage },
      { path: routes.blacklist, component: WorldLevelsPage },
      { path: routes.levels, component: WorldLevelsPage },
      { path: routes.selectorMod, component: WorldLevelsPage },
      { path: routes.preinstall, component: WorldLevelsPage },
      { path: routes.genMap, component: WorldLevelsPage },
      { path: routes.mod, component: ModPage },
      { path: routes.backup, component: BackupPage },
      { path: routes.playerLog, component: PlayerLogPage },
      { path: routes.setting, component: SettingsPage },
      { path: routes.lobby, component: LobbyPage },
      { path: routes.help, component: HelpPage },
      { path: routes.userProfile, component: UserProfilePage },
    ],
  },
]

export function createAppRouter() {
  const router = createRouter({
    history: createWebHashHistory(),
    routes: routeRecords,
  })

  router.beforeEach(async (to) => {
    if (to.meta.public) {
      return true
    }

    const auth = useAuthStore()
    if (!auth.initialized) {
      await auth.fetchCurrentUser()
    }

    if (!auth.isAuthenticated) {
      return { path: routes.login, query: { redirect: to.fullPath } }
    }

    return true
  })

  return router
}
```

- [ ] **Step 8: Wire app entry and styles**

Create `web-ui/src/app/App.vue`:

```vue
<template>
  <RouterView />
</template>
```

Create `web-ui/src/shared/styles/main.css`:

```css
@import 'element-plus/dist/index.css';

* {
  box-sizing: border-box;
}

html,
body,
#app {
  min-height: 100%;
  margin: 0;
}

body {
  color: #1f2933;
  background: #f4f6f8;
  font-family:
    Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
}

.auth-layout {
  min-height: 100vh;
  display: grid;
  place-items: center;
  background:
    linear-gradient(rgba(244, 246, 248, 0.88), rgba(244, 246, 248, 0.92)),
    url('/assets/light-bg.png') center / cover;
}

.admin-layout {
  min-height: 100vh;
}

.admin-layout__aside {
  background: #ffffff;
  border-right: 1px solid #d9e2ec;
}

.admin-layout__brand {
  height: 56px;
  display: flex;
  align-items: center;
  padding: 0 20px;
  color: #133b2a;
  font-weight: 700;
  cursor: pointer;
}

.admin-layout__menu {
  border-right: 0;
}

.admin-layout__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #ffffff;
  border-bottom: 1px solid #d9e2ec;
}

.admin-layout__actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.admin-layout__main {
  min-height: calc(100vh - 56px);
  background: #f4f6f8;
}
```

Create `web-ui/src/app/main.ts`:

```ts
import { createApp } from 'vue'

import App from './App.vue'
import { installProviders } from './providers'
import { createAppRouter } from './router'
import '@/shared/styles/main.css'

const app = createApp(App)

installProviders(app)
app.use(createAppRouter())
app.mount('#app')
```

- [ ] **Step 9: Run tests and type check**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/layout-menu.test.ts src/test/router-guard.test.ts
npm run type-check
```

Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add web-ui/src/app web-ui/src/layouts web-ui/src/shared/config web-ui/src/shared/styles web-ui/src/test/layout-menu.test.ts web-ui/src/test/router-guard.test.ts
git commit -m "feat: add vue admin shell"
```

### Task 5: Core Page Skeletons

**Files:**
- Create: `web-ui/src/shared/components/PageState.vue`
- Create: `web-ui/src/pages/BackupPage.vue`
- Create: `web-ui/src/pages/DashboardPage.vue`
- Create: `web-ui/src/pages/HelpPage.vue`
- Create: `web-ui/src/pages/InitPage.vue`
- Create: `web-ui/src/pages/LobbyPage.vue`
- Create: `web-ui/src/pages/LoginPage.vue`
- Create: `web-ui/src/pages/ModPage.vue`
- Create: `web-ui/src/pages/PanelPage.vue`
- Create: `web-ui/src/pages/PlayerLogPage.vue`
- Create: `web-ui/src/pages/SettingsPage.vue`
- Create: `web-ui/src/pages/UserProfilePage.vue`
- Create: `web-ui/src/pages/WorldLevelsPage.vue`
- Test: `web-ui/src/test/page-mount.test.ts`

- [ ] **Step 1: Write failing mount tests**

Create `web-ui/src/test/page-mount.test.ts`:

```ts
import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { describe, expect, it } from 'vitest'

import BackupPage from '@/pages/BackupPage.vue'
import DashboardPage from '@/pages/DashboardPage.vue'
import HelpPage from '@/pages/HelpPage.vue'
import InitPage from '@/pages/InitPage.vue'
import LobbyPage from '@/pages/LobbyPage.vue'
import LoginPage from '@/pages/LoginPage.vue'
import ModPage from '@/pages/ModPage.vue'
import PanelPage from '@/pages/PanelPage.vue'
import PlayerLogPage from '@/pages/PlayerLogPage.vue'
import SettingsPage from '@/pages/SettingsPage.vue'
import UserProfilePage from '@/pages/UserProfilePage.vue'
import WorldLevelsPage from '@/pages/WorldLevelsPage.vue'

const pages = [
  BackupPage,
  DashboardPage,
  HelpPage,
  InitPage,
  LobbyPage,
  LoginPage,
  ModPage,
  PanelPage,
  PlayerLogPage,
  SettingsPage,
  UserProfilePage,
  WorldLevelsPage,
]

describe('core pages', () => {
  it('mounts every route-level skeleton', () => {
    for (const page of pages) {
      const wrapper = mount(page, {
        global: {
          plugins: [createPinia()],
          stubs: {
            RouterLink: true,
            RouterView: true,
          },
        },
      })

      expect(wrapper.exists()).toBe(true)
    }
  })
})
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/page-mount.test.ts
```

Expected: FAIL because pages do not exist.

- [ ] **Step 3: Add reusable page state component**

Create `web-ui/src/shared/components/PageState.vue`:

```vue
<script setup lang="ts">
defineProps<{
  title: string
  description?: string
}>()
</script>

<template>
  <section class="page-state">
    <div class="page-state__header">
      <h1>{{ title }}</h1>
      <p v-if="description">{{ description }}</p>
    </div>
    <slot />
  </section>
</template>

<style scoped>
.page-state {
  display: grid;
  gap: 16px;
}

.page-state__header h1 {
  margin: 0;
  font-size: 22px;
  font-weight: 700;
}

.page-state__header p {
  margin: 6px 0 0;
  color: #66788a;
}
</style>
```

- [ ] **Step 4: Add auth pages**

Create `web-ui/src/pages/LoginPage.vue`:

```vue
<script setup lang="ts">
import { reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { normalizeApiError } from '@/shared/api/http'
import { useAuthStore } from '@/shared/stores/auth'

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const form = reactive({ username: '', password: '' })
const error = ref('')

async function submit(): Promise<void> {
  error.value = ''
  try {
    await auth.loginWithPassword(form)
    await router.replace((route.query.redirect as string) || '/panel')
  } catch (caught) {
    error.value = normalizeApiError(caught).message
  }
}
</script>

<template>
  <el-card class="login-card">
    <template #header>饥荒管理控制台</template>
    <el-form label-position="top" @submit.prevent="submit">
      <el-form-item label="用户名">
        <el-input v-model="form.username" autocomplete="username" />
      </el-form-item>
      <el-form-item label="密码">
        <el-input v-model="form.password" type="password" autocomplete="current-password" show-password />
      </el-form-item>
      <el-alert v-if="error" type="error" :title="error" show-icon :closable="false" />
      <el-button type="primary" :loading="auth.loading" native-type="submit" class="login-card__submit">
        登录
      </el-button>
    </el-form>
  </el-card>
</template>

<style scoped>
.login-card {
  width: min(420px, calc(100vw - 32px));
}

.login-card__submit {
  width: 100%;
  margin-top: 16px;
}
</style>
```

Create `web-ui/src/pages/InitPage.vue`:

```vue
<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
</script>

<template>
  <PageState title="初始化" description="首次运行时创建管理员账号。">
    <el-card>
      <el-alert type="info" title="初始化表单将在账户初始化增量中接入 /api/init。" :closable="false" />
    </el-card>
  </PageState>
</template>
```

- [ ] **Step 5: Add operational pages**

Create `web-ui/src/pages/DashboardPage.vue`:

```vue
<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
</script>

<template>
  <PageState title="仪表盘" description="玩家活跃、角色比例和重置统计入口。">
    <el-row :gutter="16">
      <el-col :span="8"><el-card>今日玩家</el-card></el-col>
      <el-col :span="8"><el-card>本月玩家</el-card></el-col>
      <el-col :span="8"><el-card>最近重置</el-card></el-col>
    </el-row>
  </PageState>
</template>
```

Create `web-ui/src/pages/PanelPage.vue`:

```vue
<script setup lang="ts">
import { onMounted } from 'vue'

import { useClusterStore } from '@/shared/stores/cluster'
import { useLevelStore } from '@/shared/stores/levels'
import PageState from '@/shared/components/PageState.vue'

const clusterStore = useClusterStore()
const levelStore = useLevelStore()

onMounted(() => {
  levelStore.refreshLevels(clusterStore.selectedCluster)
})
</script>

<template>
  <PageState title="面板" description="世界状态、系统信息和常用操作。">
    <el-row :gutter="16">
      <el-col :span="8"><el-card>CPU / 内存</el-card></el-col>
      <el-col :span="8"><el-card>游戏版本</el-card></el-col>
      <el-col :span="8"><el-card>存档状态</el-card></el-col>
    </el-row>
    <el-card>
      <template #header>世界状态</template>
      <el-table :data="levelStore.levels" v-loading="levelStore.loading">
        <el-table-column prop="levelName" label="世界" />
        <el-table-column prop="uuid" label="UUID" />
        <el-table-column label="操作">
          <template #default>
            <el-button size="small" disabled>启动</el-button>
            <el-button size="small" disabled>停止</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </PageState>
</template>
```

Create `web-ui/src/pages/WorldLevelsPage.vue`:

```vue
<script setup lang="ts">
import { onMounted } from 'vue'
import { useRoute } from 'vue-router'

import PageState from '@/shared/components/PageState.vue'
import { useClusterStore } from '@/shared/stores/cluster'
import { useLevelStore } from '@/shared/stores/levels'

const route = useRoute()
const clusterStore = useClusterStore()
const levelStore = useLevelStore()

onMounted(() => {
  levelStore.refreshLevels(clusterStore.selectedCluster)
})
</script>

<template>
  <PageState title="世界" :description="`当前路由：${route.path}`">
    <el-card>
      <el-table :data="levelStore.levels" v-loading="levelStore.loading">
        <el-table-column prop="levelName" label="世界名称" />
        <el-table-column prop="uuid" label="文件名" />
        <el-table-column prop="status" label="运行状态" />
      </el-table>
    </el-card>
  </PageState>
</template>
```

Create `web-ui/src/pages/ModPage.vue`:

```vue
<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
</script>

<template>
  <PageState title="模组" description="已安装模组、订阅和手动 modinfo 管理。">
    <el-card>
      <template #header>模组列表</template>
      <el-empty description="模组表格将在模组详情增量中接入真实数据。" />
    </el-card>
  </PageState>
</template>
```

Create `web-ui/src/pages/BackupPage.vue`:

```vue
<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
</script>

<template>
  <PageState title="备份" description="备份列表、上传、下载和快照设置。">
    <el-card>
      <template #header>
        <el-space>
          <span>备份列表</span>
          <el-button type="primary" disabled>创建备份</el-button>
          <el-button disabled>上传备份</el-button>
        </el-space>
      </template>
      <el-empty description="备份列表将在备份操作增量中接入真实数据。" />
    </el-card>
  </PageState>
</template>
```

Create `web-ui/src/pages/SettingsPage.vue`:

```vue
<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
</script>

<template>
  <PageState title="设置" description="DST 配置、定时任务、自动检测和主题设置。">
    <el-tabs>
      <el-tab-pane label="DST 配置"><el-empty description="配置表单增量接入 /api/dst/config。" /></el-tab-pane>
      <el-tab-pane label="定时任务"><el-empty description="任务表格增量接入 /api/task。" /></el-tab-pane>
      <el-tab-pane label="自动检测"><el-empty description="自动检测增量接入 /api/auto/check2。" /></el-tab-pane>
      <el-tab-pane label="主题"><el-empty description="主题设置增量接入本地状态。" /></el-tab-pane>
    </el-tabs>
  </PageState>
</template>
```

Create `web-ui/src/pages/PlayerLogPage.vue`:

```vue
<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
</script>

<template>
  <PageState title="玩家日志" description="玩家连接、封禁和日志查询。">
    <el-empty description="玩家日志表格增量接入 /api/player/log。" />
  </PageState>
</template>
```

Create `web-ui/src/pages/LobbyPage.vue`:

```vue
<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
</script>

<template>
  <PageState title="大厅" description="DST 大厅服务器查询。">
    <el-empty description="大厅查询增量接入 /api/dst/home/server。" />
  </PageState>
</template>
```

Create `web-ui/src/pages/HelpPage.vue`:

```vue
<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
</script>

<template>
  <PageState title="帮助" description="本地帮助文档和项目链接。">
    <el-card>
      <a href="https://github.com/yimuu/dst-panel" target="_blank" rel="noreferrer">GitHub</a>
    </el-card>
  </PageState>
</template>
```

Create `web-ui/src/pages/UserProfilePage.vue`:

```vue
<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
import { useAuthStore } from '@/shared/stores/auth'

const auth = useAuthStore()
</script>

<template>
  <PageState title="个人信息" description="当前登录管理员信息。">
    <el-card>
      <el-descriptions :column="1" border>
        <el-descriptions-item label="用户名">{{ auth.user?.username || '-' }}</el-descriptions-item>
        <el-descriptions-item label="显示昵称">{{ auth.user?.displayName || '-' }}</el-descriptions-item>
      </el-descriptions>
    </el-card>
  </PageState>
</template>
```

- [ ] **Step 6: Run tests and type check**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/page-mount.test.ts
npm run type-check
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add web-ui/src/pages web-ui/src/shared/components web-ui/src/test/page-mount.test.ts
git commit -m "feat: add core frontend pages"
```

### Task 6: Static Assets And Production Build Verification

**Files:**
- Create: `web-ui/public/favicon.ico`
- Create: `web-ui/public/Dst Emoji.woff2`
- Create: `web-ui/public/misc/*`
- Create: `web-ui/public/assets/*`
- Create: `web-ui/public/assets/dst/*`
- Modify: `web-ui/src/shared/styles/main.css`

- [ ] **Step 1: Copy stable public assets from current `dist`**

Run:

```bash
mkdir -p web-ui/public/assets web-ui/public/misc
cp -R dist/misc web-ui/public/
cp -R dist/assets/dst web-ui/public/assets/
cp dist/favicon.ico web-ui/public/favicon.ico
cp "dist/Dst Emoji.woff2" "web-ui/public/Dst Emoji.woff2"
cp dist/assets/login.png web-ui/public/assets/login.png
cp dist/assets/light-bg.png web-ui/public/assets/light-bg.png
cp dist/assets/dark-bg.png web-ui/public/assets/dark-bg.png
cp dist/assets/pig.gif web-ui/public/assets/pig.gif
cp dist/assets/fish.gif web-ui/public/assets/fish.gif
```

Expected: all copied paths exist. Do not copy hashed JavaScript or CSS chunks.

- [ ] **Step 2: Add DST emoji font CSS**

Append to `web-ui/src/shared/styles/main.css`:

```css
@font-face {
  font-family: 'Dst Emoji';
  src: url('/Dst Emoji.woff2') format('woff2');
  font-weight: normal;
  font-style: normal;
  font-display: swap;
}

.dst-emoji {
  font-family: 'Dst Emoji', sans-serif;
}
```

- [ ] **Step 3: Run full frontend verification**

Run:

```bash
cd web-ui
npm run type-check
npm run test:unit -- --run
npm run build
```

Expected: PASS. Root `dist/` contains the new Vue production build plus copied public assets.

- [ ] **Step 4: Run focused Rust static route tests**

Run:

```bash
cargo test --test compat_manifest_tests --locked
```

Expected: PASS. Static route manifest and Rust route registration remain valid.

- [ ] **Step 5: Commit**

```bash
git add web-ui/public web-ui/src/shared/styles/main.css dist
git commit -m "feat: add frontend public assets"
```

### Task 7: Browser Verification And Final Cleanup

**Files:**
- Modify only files required by verification fixes.

- [ ] **Step 1: Start the frontend dev server**

Run:

```bash
cd web-ui
npm run dev -- --host 127.0.0.1
```

Expected: Vite serves the app at `http://127.0.0.1:5173/`.

- [ ] **Step 2: Inspect the app in the browser**

Open:

```text
http://127.0.0.1:5173/#/login
```

Verify:

```text
Login page renders without a blank screen.
Sidebar and top bar render after navigating to /#/panel with a mocked or valid session.
Menu paths are visible for panel, worlds, mods, backups, settings, player log, lobby, and help.
No text overlaps at 390px mobile width or 1440px desktop width.
The console has no fatal runtime errors.
```

- [ ] **Step 3: Verify production static build through Rust when practical**

Run:

```bash
cargo run --bin dst-admin-rust
```

Open:

```text
http://127.0.0.1:8082/
```

Expected: Rust serves the generated Vue `dist/index.html`. If login requires local credentials not present in the test environment, verify at minimum that the shell loads and assets return 200 responses.

- [ ] **Step 4: Run final checks**

Run:

```bash
git status --short
cd web-ui
npm run type-check
npm run test:unit -- --run
npm run build
cd ..
cargo test --test compat_manifest_tests --locked
```

Expected: all commands pass. `git status --short` only shows intentional changes or is clean after commits.

- [ ] **Step 5: Commit verification fixes if any**

If verification required code changes:

```bash
git add web-ui dist README.md
git commit -m "fix: polish vue frontend shell"
```

If no changes were required, skip this commit.

## Self-Review Checklist

- Spec coverage: Tasks cover `web-ui/`, latest Vue/Vite dependency line, build output to `dist/`, API wrappers, Pinia stores, Chinese text constants, admin layout, menu routes, core pages, static assets, tests, and Rust static route verification.
- No backend redesign: The plan does not modify `src/` Rust modules or API route behavior.
- Type consistency: `ApiEnvelope`, `UserProfile`, `LevelSummary`, `ClusterSummary`, and store method names are used consistently across tasks.
- Execution safety: destructive actions are limited to Vite `build.emptyOutDir` writing root `dist/`, which is the intended frontend build output.
