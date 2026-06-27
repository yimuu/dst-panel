# Frontend React Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current Vue `web-ui` with a React + TypeScript + Vite frontend that preserves backend API behavior and closely matches the official DST Admin Go preview UI.

**Architecture:** Rebuild `web-ui` from a clean Vite React TypeScript scaffold, then add shared API infrastructure, React Router hash routing, TanStack Query, Ant Design Pro shell components, and route pages in visual-priority batches. Root `dist/` remains the production artifact served by Rust.

**Tech Stack:** React 19, TypeScript 6, Vite 8, Ant Design 5.29.3, Ant Design Pro Components 2.8.10, React Router 8, TanStack Query 5, Axios, Monaco Editor, Vitest, Testing Library, ESLint, Prettier.

## Global Constraints

- Preserve Rust backend routes, request parameters, response envelopes, cookies, stream paths, and static file serving behavior.
- Chinese-only UI; do not introduce i18n.
- Do not keep Vue, Element Plus, Pinia, or Vue Router in runtime dependencies.
- Do not embed the official preview's minified React bundle as application source.
- Use `antd@5.29.3`, not `antd@6.4.5`, because `@ant-design/pro-components@2.8.10` declares peer support for `antd ^4.24.15 || ^5.11.2`.
- Keep `web-ui` as the frontend source root and build production assets to root `../dist`.
- Use hash routes matching the official preview path shape.
- Root `dist/` is tracked and must be regenerated after production build.
- Required final verification: `npm run test:unit -- --run`, `npm run type-check`, `npm run lint:check`, `npm run format:check`, `npm run build`, and relevant Rust static-serving checks.

---

## File Structure

The rebuilt frontend will use this file layout:

```text
web-ui/
├── index.html
├── package.json
├── package-lock.json
├── tsconfig.app.json
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
├── public/
│   ├── favicon.ico
│   ├── assets/
│   └── misc/
└── src/
    ├── app/
    │   ├── App.tsx
    │   ├── main.tsx
    │   ├── providers.tsx
    │   └── router.tsx
    ├── layouts/
    │   ├── AdminLayout.tsx
    │   ├── AppHeader.tsx
    │   ├── AuthLayout.tsx
    │   └── menu.tsx
    ├── pages/
    │   ├── BackupPage.tsx
    │   ├── ClusterIniPage.tsx
    │   ├── DashboardPage.tsx
    │   ├── HelpPage.tsx
    │   ├── InitPage.tsx
    │   ├── LobbyPage.tsx
    │   ├── LoginPage.tsx
    │   ├── MapPreviewPage.tsx
    │   ├── ModPage.tsx
    │   ├── PanelPage.tsx
    │   ├── PlayerListPage.tsx
    │   ├── PlayerLogPage.tsx
    │   ├── PreinstallPage.tsx
    │   ├── SettingsPage.tsx
    │   ├── UserProfilePage.tsx
    │   ├── WorldLevelsPage.tsx
    │   └── WorldModSelectionPage.tsx
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
        ├── api-envelope.test.ts
        ├── api-http.test.ts
        ├── layout-menu.test.ts
        ├── page-mount.test.tsx
        ├── router-guard.test.tsx
        └── setup.ts
```

Keep `web-ui/public` assets unless a task explicitly replaces a file with the same upstream preview asset.

---

### Task 1: Clean React Scaffold And Toolchain

**Files:**
- Modify: `web-ui/package.json`
- Modify: `web-ui/package-lock.json`
- Modify: `web-ui/index.html`
- Modify: `web-ui/tsconfig.json`
- Modify: `web-ui/tsconfig.app.json`
- Modify: `web-ui/tsconfig.node.json`
- Modify: `web-ui/vite.config.ts`
- Delete: `web-ui/src` current Vue source tree
- Create: `web-ui/src/app/main.tsx`
- Create: `web-ui/src/app/App.tsx`
- Create: `web-ui/src/test/setup.ts`

**Interfaces:**
- Produces: a runnable React/Vite/TS project with scripts `dev`, `build`, `build-only`, `type-check`, `test:unit`, `lint`, `lint:check`, `format`, and `format:check`.
- Produces: `web-ui/vite.config.ts` with `build.outDir = '../dist'`, alias `@` to `web-ui/src`, and dev proxy support for backend paths.

- [ ] **Step 1: Create the scaffold outside the repo source tree**

Run:

```bash
npm create vite@latest /private/tmp/dst-panel-react-scaffold -- --template react-ts
```

Expected: Vite creates `/private/tmp/dst-panel-react-scaffold` with `src/main.tsx`, `src/App.tsx`, and React TypeScript config files.

- [ ] **Step 2: Replace Vue runtime dependencies with React dependencies**

Set `web-ui/package.json` dependencies to:

```json
{
  "@ant-design/icons": "^6.3.1",
  "@ant-design/pro-components": "^2.8.10",
  "@monaco-editor/react": "^4.7.0",
  "@tanstack/react-query": "^5.101.1",
  "antd": "^5.29.3",
  "axios": "^1.18.1",
  "monaco-editor": "^0.55.1",
  "react": "^19.2.7",
  "react-dom": "^19.2.7",
  "react-router": "^8.0.1"
}
```

Set `web-ui/package.json` dev dependencies to include:

```json
{
  "@testing-library/jest-dom": "^6.9.1",
  "@testing-library/react": "^16.3.2",
  "@types/node": "^24.13.2",
  "@types/react": "^19.2.7",
  "@types/react-dom": "^19.2.3",
  "@vitejs/plugin-react": "^6.0.3",
  "@vitest/eslint-plugin": "^1.6.20",
  "eslint": "^10.6.0",
  "eslint-config-prettier": "^10.1.8",
  "jiti": "^2.7.0",
  "jsdom": "^29.1.1",
  "prettier": "3.8.5",
  "typescript": "~6.0.3",
  "vite": "^8.1.0",
  "vitest": "^4.1.9"
}
```

- [ ] **Step 3: Install dependencies**

Run:

```bash
cd web-ui
npm install
```

Expected: `package-lock.json` is regenerated and npm exits successfully. If `react-router@8.0.1` has unusable DOM TypeScript exports during Task 3, switch to `react-router-dom@7.18.0` and document the exception in the Task 3 commit message.

- [ ] **Step 4: Configure Vite**

Create `web-ui/vite.config.ts` with this structure:

```ts
import { fileURLToPath, URL } from 'node:url'

import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
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
  build: {
    outDir: '../dist',
    emptyOutDir: true,
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
  },
})
```

- [ ] **Step 5: Create the minimal React app**

Create `web-ui/src/app/main.tsx`:

```tsx
import React from 'react'
import ReactDOM from 'react-dom/client'

import App from './App'
import '@/shared/styles/main.css'

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
```

Create `web-ui/src/app/App.tsx`:

```tsx
export default function App() {
  return <div>饥荒联机版管理面板</div>
}
```

Create `web-ui/src/test/setup.ts`:

```ts
import '@testing-library/jest-dom/vitest'
```

- [ ] **Step 6: Verify clean scaffold**

Run:

```bash
cd web-ui
npm run type-check
npm run build
```

Expected: both commands pass and root `dist/` contains React-built assets.

- [ ] **Step 7: Commit**

```bash
git add web-ui dist
git commit -m "chore: rebuild frontend scaffold with react"
```

---

### Task 2: Shared API, Config, Types, And Tests

**Files:**
- Create: `web-ui/src/shared/config/routes.ts`
- Create: `web-ui/src/shared/api/types.ts`
- Create: `web-ui/src/shared/api/envelope.ts`
- Create: `web-ui/src/shared/api/http.ts`
- Create: `web-ui/src/shared/types/domain.ts`
- Create: `web-ui/src/test/api-envelope.test.ts`
- Create: `web-ui/src/test/api-http.test.ts`

**Interfaces:**
- Produces: `routes`, a constant object with all hash route paths without `#`.
- Produces: `ApiEnvelope<T>`, `isApiSuccess()`, `readApiData()`, `assertApiSuccess()`, and `getErrorMessage()`.
- Produces: `api`, `http`, `apiGet()`, `apiPost()`, `apiPut()`, `apiDelete()`, and `setClusterHeader()`.

- [ ] **Step 1: Write API envelope tests**

Create `web-ui/src/test/api-envelope.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import {
  assertApiSuccess,
  getErrorMessage,
  isApiSuccess,
  readApiData,
} from '@/shared/api/envelope'

describe('api envelope helpers', () => {
  it('reads data from successful envelopes', () => {
    const envelope = { code: 200, data: { name: '森林' } }

    expect(isApiSuccess(envelope)).toBe(true)
    expect(readApiData(envelope)).toEqual({ name: '森林' })
    expect(assertApiSuccess(envelope)).toEqual({ name: '森林' })
  })

  it('uses message fields for failed envelopes', () => {
    expect(getErrorMessage({ code: 500, msg: '保存失败' })).toBe('保存失败')
    expect(getErrorMessage({ code: 500, message: '登录失败' })).toBe('登录失败')
  })
})
```

Create `web-ui/src/test/api-http.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { api, setClusterHeader } from '@/shared/api/http'

describe('http client', () => {
  it('sets and clears the Cluster header', () => {
    setClusterHeader('Cluster_1')
    expect(api.defaults.headers.common.Cluster).toBe('Cluster_1')

    setClusterHeader(undefined)
    expect(api.defaults.headers.common.Cluster).toBeUndefined()
  })
})
```

- [ ] **Step 2: Run failing tests**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/api-envelope.test.ts src/test/api-http.test.ts
```

Expected: fails because `shared/api/envelope.ts` and `shared/api/http.ts` do not exist yet.

- [ ] **Step 3: Implement envelope helpers**

Create `web-ui/src/shared/api/types.ts`:

```ts
export interface ApiEnvelope<T = unknown> {
  code?: number
  data?: T
  msg?: string
  message?: string
  [key: string]: unknown
}
```

Create `web-ui/src/shared/api/envelope.ts`:

```ts
import type { ApiEnvelope } from './types'

export function isApiSuccess(response: ApiEnvelope<unknown>): boolean {
  return response.code === 200 || response.code === 0
}

export function readApiData<T>(response: ApiEnvelope<T>): T {
  return response.data as T
}

export function getErrorMessage(error: unknown, fallback = '请求失败'): string {
  if (typeof error === 'string' && error.trim()) {
    return error
  }

  if (typeof error === 'object' && error !== null) {
    const record = error as Record<string, unknown>
    const response = record.response as { data?: Record<string, unknown> } | undefined
    const data = response?.data ?? record
    const msg = data.msg
    const message = data.message

    if (typeof msg === 'string' && msg.trim()) {
      return msg
    }

    if (typeof message === 'string' && message.trim()) {
      return message
    }
  }

  return fallback
}

export function assertApiSuccess<T>(response: ApiEnvelope<T>): T {
  if (!isApiSuccess(response)) {
    throw new Error(getErrorMessage(response))
  }

  return readApiData(response)
}
```

- [ ] **Step 4: Add route config**

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

export type AppRoutePath = (typeof routes)[keyof typeof routes]
```

- [ ] **Step 5: Implement HTTP client**

Create `web-ui/src/shared/api/http.ts`:

```ts
import axios, { type AxiosRequestConfig } from 'axios'

export const api = axios.create({
  baseURL: '/',
  withCredentials: true,
})

export const http = api

export function setClusterHeader(cluster: string | undefined): void {
  if (cluster) {
    api.defaults.headers.common.Cluster = cluster
    return
  }

  delete api.defaults.headers.common.Cluster
}

export async function apiGet<T>(url: string, config?: AxiosRequestConfig): Promise<T> {
  const response = await api.get<T>(url, config)
  return response.data
}

export async function apiPost<T, P = unknown>(
  url: string,
  payload?: P,
  config?: AxiosRequestConfig,
): Promise<T> {
  const response = await api.post<T>(url, payload, config)
  return response.data
}

export async function apiPut<T, P = unknown>(
  url: string,
  payload?: P,
  config?: AxiosRequestConfig,
): Promise<T> {
  const response = await api.put<T>(url, payload, config)
  return response.data
}

export async function apiDelete<T>(url: string, config?: AxiosRequestConfig): Promise<T> {
  const response = await api.delete<T>(url, config)
  return response.data
}
```

- [ ] **Step 6: Add domain types**

Create `web-ui/src/shared/types/domain.ts`:

```ts
export interface CurrentUser {
  id?: string | number
  ID?: string | number
  username?: string
  name?: string
  displayName?: string
  role?: string
  createdAt?: string
  created_at?: string
}

export interface LevelSummary {
  uuid?: string
  name?: string
  levelName?: string
  is_master?: boolean
  status?: boolean
  [key: string]: unknown
}

export interface DstConfig {
  steamcmd: string
  force_install_dir: string
  backup: string
  mod_download_path: string
  cluster: string
  persistent_storage_root: string
  conf_dir: string
  ugc_directory: string
  donot_starve_server_directory: string
  bin: '32' | '64'
  beta: 0 | 1
}
```

- [ ] **Step 7: Verify task**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/api-envelope.test.ts src/test/api-http.test.ts
npm run type-check
```

Expected: tests and type-check pass.

- [ ] **Step 8: Commit**

```bash
git add web-ui/src/shared web-ui/src/test/api-envelope.test.ts web-ui/src/test/api-http.test.ts
git commit -m "feat: add react frontend api foundation"
```

---

### Task 3: Auth APIs, Router, Providers, And Ant Design Pro Shell

**Files:**
- Create: `web-ui/src/app/providers.tsx`
- Modify: `web-ui/src/app/App.tsx`
- Create: `web-ui/src/app/router.tsx`
- Create: `web-ui/src/features/auth/auth.api.ts`
- Create: `web-ui/src/features/auth/auth-state.ts`
- Create: `web-ui/src/layouts/AdminLayout.tsx`
- Create: `web-ui/src/layouts/AuthLayout.tsx`
- Create: `web-ui/src/layouts/AppHeader.tsx`
- Create: `web-ui/src/layouts/menu.tsx`
- Create: `web-ui/src/shared/styles/main.css`
- Create: `web-ui/src/test/layout-menu.test.ts`
- Create: `web-ui/src/test/router-guard.test.tsx`
- Create: `web-ui/src/pages/BackupPage.tsx`
- Create: `web-ui/src/pages/ClusterIniPage.tsx`
- Create: `web-ui/src/pages/DashboardPage.tsx`
- Create: `web-ui/src/pages/HelpPage.tsx`
- Create: `web-ui/src/pages/InitPage.tsx`
- Create: `web-ui/src/pages/LobbyPage.tsx`
- Create: `web-ui/src/pages/LoginPage.tsx`
- Create: `web-ui/src/pages/MapPreviewPage.tsx`
- Create: `web-ui/src/pages/ModPage.tsx`
- Create: `web-ui/src/pages/PanelPage.tsx`
- Create: `web-ui/src/pages/PlayerListPage.tsx`
- Create: `web-ui/src/pages/PlayerLogPage.tsx`
- Create: `web-ui/src/pages/PreinstallPage.tsx`
- Create: `web-ui/src/pages/SettingsPage.tsx`
- Create: `web-ui/src/pages/UserProfilePage.tsx`
- Create: `web-ui/src/pages/WorldLevelsPage.tsx`
- Create: `web-ui/src/pages/WorldModSelectionPage.tsx`

**Interfaces:**
- Consumes: `routes`, `api`, `ApiEnvelope`, `CurrentUser`.
- Produces: auth API functions `getInitStatus()`, `login()`, `logout()`, `getCurrentUser()`.
- Produces: `getAuthRedirect()` for route guard decisions.
- Produces: `adminMenuItems` and `flattenAdminMenuItems()`.
- Produces: protected hash routing and an official-preview-style Ant Design Pro shell.

- [ ] **Step 1: Write menu tests**

Create `web-ui/src/test/layout-menu.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { adminMenuItems, flattenAdminMenuItems } from '@/layouts/menu'
import { routes } from '@/shared/config/routes'

describe('admin menu', () => {
  it('contains official preview route groups', () => {
    const paths = flattenAdminMenuItems(adminMenuItems).map((item) => item.path)

    expect(paths).toContain(routes.dashboard)
    expect(paths).toContain(routes.panel)
    expect(paths).toContain(routes.clusterIni)
    expect(paths).toContain(routes.levels)
    expect(paths).toContain(routes.mod)
    expect(paths).toContain(routes.backup)
  })
})
```

- [ ] **Step 2: Run failing menu test**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/layout-menu.test.ts
```

Expected: fails because `layouts/menu.tsx` does not exist yet.

- [ ] **Step 3: Write route guard tests**

Create `web-ui/src/test/router-guard.test.tsx`:

```tsx
import { describe, expect, it } from 'vitest'

import { getAuthRedirect } from '@/features/auth/auth-state'
import { routes } from '@/shared/config/routes'

describe('auth route decisions', () => {
  it('sends first-run users to init', () => {
    expect(
      getAuthRedirect({
        firstRun: true,
        authenticated: false,
        publicRoute: false,
        path: routes.panel,
      }),
    ).toBe(routes.init)
  })

  it('sends anonymous protected users to login', () => {
    expect(
      getAuthRedirect({
        firstRun: false,
        authenticated: false,
        publicRoute: false,
        path: routes.panel,
      }),
    ).toBe(routes.login)
  })

  it('allows authenticated protected routes', () => {
    expect(
      getAuthRedirect({
        firstRun: false,
        authenticated: true,
        publicRoute: false,
        path: routes.panel,
      }),
    ).toBeUndefined()
  })
})
```

- [ ] **Step 4: Implement auth API and auth state model**

Create `web-ui/src/features/auth/auth.api.ts`:

```ts
import { api } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { CurrentUser } from '@/shared/types/domain'

export interface LoginPayload {
  username: string
  password: string
}

export async function getInitStatus(): Promise<ApiEnvelope<unknown>> {
  const response = await api.get<ApiEnvelope<unknown>>('/api/init')
  return response.data
}

export async function login(payload: LoginPayload): Promise<ApiEnvelope<CurrentUser>> {
  const response = await api.post<ApiEnvelope<CurrentUser>>('/api/login', payload)
  return response.data
}

export async function logout(): Promise<ApiEnvelope<unknown>> {
  const response = await api.post<ApiEnvelope<unknown>>('/api/logout')
  return response.data
}

export async function getCurrentUser(): Promise<ApiEnvelope<CurrentUser>> {
  const response = await api.get<ApiEnvelope<CurrentUser>>('/api/user')
  return response.data
}
```

Create `web-ui/src/features/auth/auth-state.ts`:

```ts
import { routes } from '@/shared/config/routes'

export interface AuthRouteDecision {
  firstRun: boolean
  authenticated: boolean
  publicRoute: boolean
  path: string
}

export function getAuthRedirect(decision: AuthRouteDecision): string | undefined {
  if (decision.firstRun && decision.path !== routes.init) {
    return routes.init
  }

  if (!decision.firstRun && decision.path === routes.init) {
    return routes.login
  }

  if (decision.publicRoute) {
    return undefined
  }

  return decision.authenticated ? undefined : routes.login
}
```

- [ ] **Step 5: Implement menu**

Create `web-ui/src/layouts/menu.tsx`:

```tsx
import type { MenuDataItem } from '@ant-design/pro-components'
import {
  CloudServerOutlined,
  DashboardOutlined,
  FileProtectOutlined,
  GithubOutlined,
  HomeOutlined,
  ProfileOutlined,
  ReadOutlined,
  SettingOutlined,
  TeamOutlined,
  ToolOutlined,
} from '@ant-design/icons'

import { routes } from '@/shared/config/routes'

export interface AdminMenuItem extends MenuDataItem {
  path: string
  name: string
  children?: AdminMenuItem[]
}

export const adminMenuItems: AdminMenuItem[] = [
  { path: routes.dashboard, name: 'Dashboard', icon: <DashboardOutlined /> },
  { path: routes.panel, name: '面板操作', icon: <CloudServerOutlined /> },
  {
    path: routes.clusterIni,
    name: '房间设置',
    icon: <HomeOutlined />,
    children: [
      { path: routes.clusterIni, name: '房间设置' },
      { path: routes.adminlist, name: '管理员列表' },
      { path: routes.whitelist, name: '白名单列表' },
      { path: routes.blacklist, name: '黑名单列表' },
    ],
  },
  {
    path: routes.levels,
    name: '世界设置',
    icon: <ToolOutlined />,
    children: [
      { path: routes.levels, name: '世界设置' },
      { path: routes.selectorMod, name: '多层选择器' },
      { path: routes.preinstall, name: '世界模板' },
      { path: routes.genMap, name: '预览地图' },
    ],
  },
  { path: routes.mod, name: '模组设置', icon: <ProfileOutlined /> },
  { path: routes.backup, name: '存档备份', icon: <FileProtectOutlined /> },
  { path: routes.playerLog, name: '玩家日志', icon: <TeamOutlined /> },
  { path: routes.setting, name: '系统设置', icon: <SettingOutlined /> },
  { path: routes.lobby, name: '大厅列表', icon: <CloudServerOutlined /> },
  { path: routes.help, name: '帮助文档', icon: <ReadOutlined /> },
  { path: 'https://github.com/carrot-hu23/dst-admin-go', name: 'Github', icon: <GithubOutlined /> },
]

export function flattenAdminMenuItems(items: AdminMenuItem[] = adminMenuItems): AdminMenuItem[] {
  return items.flatMap((item) => (item.children ? flattenAdminMenuItems(item.children) : [item]))
}
```

- [ ] **Step 6: Implement providers**

Create `web-ui/src/app/providers.tsx`:

```tsx
import type { PropsWithChildren } from 'react'
import { ConfigProvider, App as AntApp } from 'antd'
import zhCN from 'antd/locale/zh_CN'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
      refetchOnWindowFocus: false,
    },
  },
})

export function AppProviders({ children }: PropsWithChildren) {
  return (
    <QueryClientProvider client={queryClient}>
      <ConfigProvider
        locale={zhCN}
        theme={{
          token: {
            colorPrimary: '#4f46e5',
            borderRadius: 8,
          },
          components: {
            Card: { borderRadiusLG: 16 },
          },
        }}
      >
        <AntApp>{children}</AntApp>
      </ConfigProvider>
    </QueryClientProvider>
  )
}
```

- [ ] **Step 7: Implement shell and route page shells**

Create `AdminLayout.tsx` with `ProLayout` using `adminMenuItems`, a `v1.6.1` tag, white sidebar/header, and `<Outlet />`.

Create page shell components that render a `ProCard` with the correct Chinese page title. Use exact page names from the file list. For example `DashboardPage.tsx`:

```tsx
import { ProCard } from '@ant-design/pro-components'

export default function DashboardPage() {
  return <ProCard title="Dashboard" bordered={false}>统计数据加载中</ProCard>
}
```

Create `LoginPage.tsx` with Ant Design `Form`, username/password fields, and a submit button labeled `登录`.

- [ ] **Step 8: Implement router**

Create `web-ui/src/app/router.tsx` with hash routing, default redirect to `routes.panel`, auth layout routes for login/init, and admin layout routes for protected pages. Use `createHashRouter`, `Navigate`, `Outlet`, and `RouterProvider` from React Router APIs available after install.

If `react-router@8.0.1` DOM exports are not usable in TypeScript, replace the dependency with `react-router-dom@7.18.0`, import DOM APIs from `react-router-dom`, and record the change in the commit body.

- [ ] **Step 9: Add global styles**

Create `web-ui/src/shared/styles/main.css`:

```css
* {
  box-sizing: border-box;
}

html,
body,
#root {
  min-height: 100vh;
  margin: 0;
}

body {
  color: #1f1f1f;
  background: #f0f2f5;
  font-family:
    -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei',
    sans-serif;
}

.admin-page {
  min-height: calc(100vh - 56px);
  padding: 24px;
  background: #f0f2f5;
}

.ant-pro-card {
  border-radius: 16px;
}
```

- [ ] **Step 10: Verify shell**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/layout-menu.test.ts src/test/router-guard.test.tsx
npm run type-check
npm run build
```

Expected: tests, type-check, and build pass.

- [ ] **Step 11: Commit**

```bash
git add web-ui dist
git commit -m "feat: add react ant design shell"
```

---

### Task 4: Dashboard And Panel Official-Style First Pass

**Files:**
- Create: `web-ui/src/features/dashboard/dashboard-model.ts`
- Create: `web-ui/src/features/statistics/statistics.api.ts`
- Create: `web-ui/src/features/panel/panel-model.ts`
- Create: `web-ui/src/features/game/game.api.ts`
- Modify: `web-ui/src/pages/DashboardPage.tsx`
- Modify: `web-ui/src/pages/PanelPage.tsx`
- Create: `web-ui/src/test/dashboard-model.test.ts`
- Create: `web-ui/src/test/panel-model.test.ts`

**Interfaces:**
- Produces: dashboard stat model functions with chart-friendly arrays.
- Produces: panel action model functions `getLevelActionTarget()` and `getPanelActionLabel()`.
- Produces: Dashboard and Panel pages visually aligned with `docs/image/dashboard.png` and `docs/image/panel.png`.

- [ ] **Step 1: Write model tests**

Create `web-ui/src/test/dashboard-model.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { buildDashboardSummaryCards } from '@/features/dashboard/dashboard-model'

describe('dashboard model', () => {
  it('builds official summary cards', () => {
    expect(buildDashboardSummaryCards({ todayOnline: 1, monthOnline: 2 })).toEqual([
      { title: '今日在线人数', value: 1, color: '#1677ff' },
      { title: '本月在线人数', value: 2, color: '#f5c542' },
    ])
  })
})
```

Create `web-ui/src/test/panel-model.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { getLevelActionTarget, getPanelActionLabel } from '@/features/panel/panel-model'

describe('panel model', () => {
  it('formats action labels and level targets', () => {
    expect(getPanelActionLabel('start')).toBe('启动世界')
    expect(getPanelActionLabel('stop')).toBe('停止世界')
    expect(getPanelActionLabel('restart')).toBe('重启世界')
    expect(getLevelActionTarget({ levelName: '森林' })).toBe('森林')
    expect(getLevelActionTarget({ name: '洞穴' })).toBe('洞穴')
  })
})
```

- [ ] **Step 2: Run failing test**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/dashboard-model.test.ts src/test/panel-model.test.ts
```

Expected: fails because `dashboard-model.ts` and `panel-model.ts` do not exist yet.

- [ ] **Step 3: Implement dashboard and panel models**

Create `web-ui/src/features/dashboard/dashboard-model.ts`:

```ts
export interface DashboardSummaryInput {
  todayOnline: number
  monthOnline: number
}

export interface DashboardSummaryCard {
  title: string
  value: number
  color: string
}

export function buildDashboardSummaryCards(input: DashboardSummaryInput): DashboardSummaryCard[] {
  return [
    { title: '今日在线人数', value: input.todayOnline, color: '#1677ff' },
    { title: '本月在线人数', value: input.monthOnline, color: '#f5c542' },
  ]
}
```

Create `web-ui/src/features/panel/panel-model.ts`:

```ts
import type { LevelSummary } from '@/shared/types/domain'

export type PanelAction = 'start' | 'stop' | 'restart'

export function getPanelActionLabel(action: PanelAction): string {
  return {
    start: '启动世界',
    stop: '停止世界',
    restart: '重启世界',
  }[action]
}

export function getLevelActionTarget(level: LevelSummary): string {
  return level.levelName || level.name || ''
}
```

- [ ] **Step 4: Implement game and statistics APIs**

Create `web-ui/src/features/game/game.api.ts` with wrappers for:

- `GET /api/game/8level/status`
- `GET /api/game/8level/start?levelName=...`
- `GET /api/game/8level/stop?levelName=...`
- `POST /api/game/8level/command`
- `GET /api/game/system/info`
- `GET /api/game/update`
- `POST /api/game/backup`

Create `web-ui/src/features/statistics/statistics.api.ts` with wrappers for:

- `GET /api/statistics/active/user/?unit=...`
- `GET /api/statistics/rate/role/?&startDate=...`
- `GET /api/statistics/top/active/?N=...`
- `GET /api/statistics/regenerate?N=...`

- [ ] **Step 5: Rebuild Dashboard layout**

Update `DashboardPage.tsx` to use:

- `ProCard` date toolbar.
- Two statistic cards for `今日在线人数` and `本月在线人数`.
- Two-column grid with chart cards titled `本周玩家活跃情况` and `本周角色比例`.
- Bottom cards titled `本周前十玩家排名` and `重置时间线`.

Use Ant Design `Statistic`, `Segmented`, `DatePicker.RangePicker`, `Empty`, and simple CSS chart stand-ins if chart library is not introduced in this task. Preserve the official card spacing and light gray page background.

- [ ] **Step 6: Rebuild Panel layout**

Update `PanelPage.tsx` to use:

- Top `Tabs` with `面板`, `远程`, `TooManyItemsPlus`, `自定义指令`, `自定义指令-编辑`.
- Resource summary `ProCard` row with panel version, memory, CPU, and disk usage.
- Two-column content grid.
- Left cards: `服务器信息`, `世界列表`.
- Right cards: `服务器日志`, `玩家列表`.
- Log area in monospace with command input and rollback action buttons.

- [ ] **Step 7: Verify task**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/dashboard-model.test.ts src/test/panel-model.test.ts
npm run type-check
npm run lint:check
npm run build
```

Expected: all commands pass.

- [ ] **Step 8: Browser visual check**

Run dev server:

```bash
cd web-ui
npm run dev -- --host 127.0.0.1
```

Open `http://127.0.0.1:5173/#/dashboard` and `http://127.0.0.1:5173/#/panel`. Compare against `docs/image/dashboard.png` and `docs/image/panel.png`.

- [ ] **Step 9: Commit**

```bash
git add web-ui dist
git commit -m "feat: rebuild dashboard and panel pages"
```

---

### Task 5: Room Settings And Player Lists

**Files:**
- Create: `web-ui/src/features/clusters/cluster.api.ts`
- Create: `web-ui/src/features/room/room.api.ts`
- Create: `web-ui/src/features/room/player-lists.ts`
- Modify: `web-ui/src/pages/ClusterIniPage.tsx`
- Modify: `web-ui/src/pages/PlayerListPage.tsx`
- Create: `web-ui/src/test/player-lists.test.ts`

**Interfaces:**
- Produces: room config API wrappers around `/api/game/8level/clusterIni`.
- Produces: list type model for `adminlist`, `whitelist`, and `blacklist`.
- Produces: official-style room settings page matching `docs/image/home.png`.

- [ ] **Step 1: Write player list model tests**

Create `web-ui/src/test/player-lists.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { getPlayerListTitle } from '@/features/room/player-lists'

describe('player list labels', () => {
  it('maps list kinds to official titles', () => {
    expect(getPlayerListTitle('adminlist')).toBe('管理员列表')
    expect(getPlayerListTitle('whitelist')).toBe('白名单列表')
    expect(getPlayerListTitle('blacklist')).toBe('黑名单列表')
  })
})
```

- [ ] **Step 2: Implement list model**

Create `web-ui/src/features/room/player-lists.ts`:

```ts
export type PlayerListKind = 'adminlist' | 'whitelist' | 'blacklist'

export function getPlayerListTitle(kind: PlayerListKind): string {
  return {
    adminlist: '管理员列表',
    whitelist: '白名单列表',
    blacklist: '黑名单列表',
  }[kind]
}
```

- [ ] **Step 3: Implement room APIs**

Create API wrappers for:

- `GET /api/game/8level/clusterIni`
- `POST /api/game/8level/clusterIni`
- `GET /api/game/8level/adminilist`
- `GET /api/game/8level/whitelist`
- `GET /api/game/8level/blacklist`
- player add/delete endpoints already exposed by the Rust handlers.

- [ ] **Step 4: Rebuild ClusterIniPage**

Use Ant Design `Form` with horizontal labels, matching `docs/image/home.png`:

- Section title `基础设置`.
- Fields `名称`, `描述`, `游戏模式`, `最大玩家数`, `密码`, `令牌`, `PVP`, `投票启用`, `无人时暂停`, `控制台启用`, `白名单名额`.
- Fixed bottom save action aligned right.
- Tooltips next to labels where the official screenshot shows help markers.

- [ ] **Step 5: Rebuild PlayerListPage**

Use `ProTable` or Ant Design `Table` with title, add input, refresh button, and delete actions. The page must share one implementation through `kind: PlayerListKind` route props.

- [ ] **Step 6: Verify task**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/player-lists.test.ts
npm run type-check
npm run lint:check
npm run build
```

Expected: all commands pass.

- [ ] **Step 7: Commit**

```bash
git add web-ui dist
git commit -m "feat: rebuild room settings pages"
```

---

### Task 6: World Settings, Selector Mod, Preinstall, And Map Preview

**Files:**
- Create: `web-ui/src/features/levels/level.api.ts`
- Create: `web-ui/src/features/worlds/world-settings-model.ts`
- Create: `web-ui/src/features/mods/mod-selection.ts`
- Create: `web-ui/src/features/maps/map.api.ts`
- Create: `web-ui/src/features/maps/map-state.ts`
- Modify: `web-ui/src/pages/WorldLevelsPage.tsx`
- Modify: `web-ui/src/pages/WorldModSelectionPage.tsx`
- Modify: `web-ui/src/pages/PreinstallPage.tsx`
- Modify: `web-ui/src/pages/MapPreviewPage.tsx`
- Create: `web-ui/src/test/world-settings-model.test.ts`
- Create: `web-ui/src/test/mod-selection.test.ts`

**Interfaces:**
- Produces: world settings parsing/render model from `misc/dst_world_setting.json`.
- Produces: world tabs and grouped setting grid using DST images.
- Produces: selector mod, preinstall, and map preview pages with official-style page chrome.

- [ ] **Step 1: Write world settings tests**

Create `web-ui/src/test/world-settings-model.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { normalizeWorldOptionValue } from '@/features/worlds/world-settings-model'

describe('world settings model', () => {
  it('keeps known values and falls back to default display values', () => {
    expect(normalizeWorldOptionValue('often')).toBe('often')
    expect(normalizeWorldOptionValue(undefined)).toBe('default')
  })
})
```

Create `web-ui/src/test/mod-selection.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { toggleSelectedMod } from '@/features/mods/mod-selection'

describe('world mod selection', () => {
  it('adds and removes selected mod ids', () => {
    expect(toggleSelectedMod(['378160973'], '123')).toEqual(['378160973', '123'])
    expect(toggleSelectedMod(['378160973', '123'], '123')).toEqual(['378160973'])
  })
})
```

- [ ] **Step 2: Implement world and mod-selection models**

Create `web-ui/src/features/worlds/world-settings-model.ts`:

```ts
export function normalizeWorldOptionValue(value: string | undefined): string {
  return value && value.trim() ? value : 'default'
}
```

Create `web-ui/src/features/mods/mod-selection.ts`:

```ts
export function toggleSelectedMod(selectedIds: string[], modId: string): string[] {
  return selectedIds.includes(modId)
    ? selectedIds.filter((selectedId) => selectedId !== modId)
    : [...selectedIds, modId]
}
```

- [ ] **Step 3: Implement level and map APIs**

Create wrappers for:

- `GET /api/cluster/level`
- `POST /api/cluster/level`
- `DELETE /api/cluster/level?levelName=...`
- `GET /api/dst-static/dst_world_setting.json`
- `GET /api/dst-static/worldgen_customization.webp`
- `GET /api/dst-static/worldsettings_customization.webp`
- `GET /api/dst/map/image?clusterName=...`
- `GET /api/dst/map/gen?clusterName=...`

- [ ] **Step 4: Rebuild WorldLevelsPage**

Match `docs/image/level.png`:

- Blue info `Alert`.
- Level tabs `森林`, `洞穴`.
- Nested tabs `世界设置`, `模组设置`, `端口设置`.
- View/edit tabs `查看`, `编辑`.
- Secondary tabs `世界设置`, `世界生成`.
- Grid of image + label + Select controls.
- Bottom action buttons `保存`, `添加世界`, `导入`, `下载`.

- [ ] **Step 5: Rebuild selector/preinstall/map pages**

Use official-style `ProCard`, `Tabs`, and action bars:

- `WorldModSelectionPage`: selector mod configuration layout.
- `PreinstallPage`: template cards from `public/misc/preinstall.json`.
- `MapPreviewPage`: map image card, generate button, refresh button, empty/loading/error states.

- [ ] **Step 6: Verify task**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/world-settings-model.test.ts src/test/mod-selection.test.ts
npm run type-check
npm run lint:check
npm run build
```

Expected: all commands pass.

- [ ] **Step 7: Browser visual check**

Open `http://127.0.0.1:5173/#/levels/levels` and compare against `docs/image/level.png`.

- [ ] **Step 8: Commit**

```bash
git add web-ui dist
git commit -m "feat: rebuild world settings pages"
```

---

### Task 7: Mod Settings

**Files:**
- Create: `web-ui/src/features/mods/mod.api.ts`
- Create: `web-ui/src/features/mods/mod-model.ts`
- Modify: `web-ui/src/pages/ModPage.tsx`
- Create: `web-ui/src/test/mod-model.test.ts`

**Interfaces:**
- Produces: mod API wrappers around `/api/mod`, `/api/mod/search`, `/api/mod/modinfo`, `/api/mod/ugc`.
- Produces: model helpers for selected mod display and enabled status.
- Produces: Mod page matching `docs/image/mod1.png`, `docs/image/mod2.png`, and `docs/image/mod3.png`.

- [ ] **Step 1: Write mod model tests**

Create `web-ui/src/test/mod-model.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { formatWorkshopId, isModEnabled } from '@/features/mods/mod-model'

describe('mod model', () => {
  it('formats workshop ids and enabled state', () => {
    expect(formatWorkshopId('workshop-378160973')).toBe('378160973')
    expect(formatWorkshopId('378160973')).toBe('378160973')
    expect(isModEnabled({ enabled: true })).toBe(true)
    expect(isModEnabled({ enabled: false })).toBe(false)
  })
})
```

- [ ] **Step 2: Implement mod model**

Create `web-ui/src/features/mods/mod-model.ts`:

```ts
export interface ModSummary {
  enabled?: boolean
  [key: string]: unknown
}

export function formatWorkshopId(value: string): string {
  return value.replace(/^workshop-/, '')
}

export function isModEnabled(mod: ModSummary): boolean {
  return mod.enabled === true
}
```

- [ ] **Step 3: Implement mod APIs**

Create wrappers for:

- `GET /api/mod`
- `POST /api/mod`
- `GET /api/mod/search?text=...`
- `POST /api/mod/modinfo`
- `POST /api/mod/modinfo/file`
- `GET /api/mod/ugc?levelName=...`
- `GET /api/mod/ugc/acf?levelName=...`

- [ ] **Step 4: Rebuild ModPage**

Use:

- Top tabs `模组设置`, `模组订阅`, `Ugc模组`.
- Blue info `Alert`.
- Toolbar buttons `保存`, `全部更新`, `上传自定义模组配置`, level select, `保存到森林`.
- Left selectable mod list cards with image, title, enabled switch, delete action.
- Right details panel with image, name, version, workshop id, author, last update, compatibility, description.
- Bottom action bar `选项`, `更新`, `创意工坊`.

- [ ] **Step 5: Verify task**

Run:

```bash
cd web-ui
npm run test:unit -- --run src/test/mod-model.test.ts
npm run type-check
npm run lint:check
npm run build
```

Expected: all commands pass.

- [ ] **Step 6: Browser visual check**

Open `http://127.0.0.1:5173/#/mod` and compare against `docs/image/mod1.png`.

- [ ] **Step 7: Commit**

```bash
git add web-ui dist
git commit -m "feat: rebuild mod settings page"
```

---

### Task 8: Backup, Player Log, Settings, Lobby, Help, Login, Init, Profile

**Files:**
- Create: `web-ui/src/features/backups/backup.api.ts`
- Create: `web-ui/src/features/backups/backup-format.ts`
- Create: `web-ui/src/features/settings/settings.api.ts`
- Create: `web-ui/src/features/settings/settings-form.ts`
- Create: `web-ui/src/features/auth/user-profile.ts`
- Modify: `web-ui/src/pages/BackupPage.tsx`
- Modify: `web-ui/src/pages/PlayerLogPage.tsx`
- Modify: `web-ui/src/pages/SettingsPage.tsx`
- Modify: `web-ui/src/pages/LobbyPage.tsx`
- Modify: `web-ui/src/pages/HelpPage.tsx`
- Modify: `web-ui/src/pages/LoginPage.tsx`
- Modify: `web-ui/src/pages/InitPage.tsx`
- Modify: `web-ui/src/pages/UserProfilePage.tsx`
- Create: `web-ui/src/test/backup-format.test.ts`
- Create: `web-ui/src/test/settings-form.test.ts`
- Create: `web-ui/src/test/user-profile.test.ts`
- Create: `web-ui/src/test/page-mount.test.tsx`

**Interfaces:**
- Produces: remaining route pages with Ant Design visual consistency.
- Produces: profile/settings validation helpers carried forward from the previous Vue implementation.
- Produces: page mount coverage for all configured routes.

- [ ] **Step 1: Port helper tests**

Create `web-ui/src/test/settings-form.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { validateDstConfig } from '@/features/settings/settings-form'

describe('settings form', () => {
  it('rejects missing required fields', () => {
    expect(validateDstConfig({ steamcmd: '', force_install_dir: '', backup: '', mod_download_path: '', cluster: '', persistent_storage_root: '', conf_dir: '', ugc_directory: '', donot_starve_server_directory: '', bin: '32', beta: 0 })).toContain('steamcmd')
  })
})
```

Create `web-ui/src/test/user-profile.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { normalizeUserProfileName, validateNewPassword } from '@/features/auth/user-profile'

describe('user profile helpers', () => {
  it('normalizes names and password input', () => {
    expect(normalizeUserProfileName({ displayName: 'admin' })).toBe('admin')
    expect(validateNewPassword(' 12345 ')).toBe('密码长度至少 6 位')
    expect(validateNewPassword(' 123456 ')).toBeUndefined()
  })
})
```

Create `web-ui/src/test/backup-format.test.ts`:

```ts
import { describe, expect, it } from 'vitest'

import { formatBackupSize } from '@/features/backups/backup-format'

describe('backup format', () => {
  it('formats byte sizes for backup tables', () => {
    expect(formatBackupSize(0)).toBe('0 B')
    expect(formatBackupSize(1024)).toBe('1.00 KB')
    expect(formatBackupSize(1048576)).toBe('1.00 MB')
  })
})
```

- [ ] **Step 2: Implement helpers**

Create focused helper files:

- `settings-form.ts` exports `validateDstConfig(config: DstConfig): string[]`.
- `user-profile.ts` exports `normalizeUserProfileName(user: CurrentUser | undefined): string` and `validateNewPassword(value: string): string | undefined`.
- `backup-format.ts` exports `formatBackupSize(bytes: number): string`.

`backup-format.ts` must use these units in order: `B`, `KB`, `MB`, `GB`.

- [ ] **Step 3: Implement remaining APIs**

Create wrappers for:

- Backup: `GET /api/game/backup`, `POST /api/game/backup`, `PUT /api/game/backup`, `DELETE /api/game/backup`, `GET /api/game/backup/restore?backupName=...`, `POST /api/game/backup/upload`, `GET /api/game/backup/download?fileName=...`, `GET /api/game/backup/snapshot/setting`, `POST /api/game/backup/snapshot/setting`.
- Player log: `GET /api/player/log?`, `POST /api/player/log/delete`, `POST /api/game/player/blacklist`.
- Settings: `GET /api/dst/config`, `POST /api/dst/config`, `GET /api/task`, `POST /api/task`, `DELETE /api/task?jobId=...`, `GET /api/web/link`, `POST /api/web/link`, `DELETE /api/web/link?ID=...`.
- Lobby: `GET /api/dst/home/server`, `GET /api/dst/home/server/detail`.
- Help: static `GET /misc/FQA.md`, `GET /misc/Docker-compose.md`, `GET /misc/DontStarveMultiWorldTotorial.md`, `GET /misc/DontStarveServerMultipleMachinesSeriesTutorial.md`.

- [ ] **Step 4: Rebuild remaining pages**

Use Ant Design components:

- `BackupPage`: `ProTable`, upload button, create backup, snapshot settings card.
- `PlayerLogPage`: filters, table, delete/block actions.
- `SettingsPage`: form sections for DST config, timed task, and web link settings using the APIs listed in Step 3.
- `LobbyPage`: table/list with query controls.
- `HelpPage`: Markdown-like help content from `public/misc`.
- `LoginPage`: official-style login background and compact form.
- `InitPage`: first-run setup form.
- `UserProfilePage`: account info card and password form.

- [ ] **Step 5: Add page mount tests**

Create `web-ui/src/test/page-mount.test.tsx`:

```tsx
import { render, screen } from '@testing-library/react'
import type { ReactElement } from 'react'
import { describe, expect, it } from 'vitest'

import BackupPage from '@/pages/BackupPage'
import HelpPage from '@/pages/HelpPage'
import LobbyPage from '@/pages/LobbyPage'
import LoginPage from '@/pages/LoginPage'
import PlayerLogPage from '@/pages/PlayerLogPage'
import SettingsPage from '@/pages/SettingsPage'
import UserProfilePage from '@/pages/UserProfilePage'
import { AppProviders } from '@/app/providers'

function renderPage(page: ReactElement) {
  return render(<AppProviders>{page}</AppProviders>)
}

describe('remaining route pages', () => {
  it('mounts the remaining page titles', () => {
    renderPage(<BackupPage />)
    expect(screen.getByText('存档备份')).toBeInTheDocument()

    renderPage(<PlayerLogPage />)
    expect(screen.getByText('玩家日志')).toBeInTheDocument()

    renderPage(<SettingsPage />)
    expect(screen.getByText('系统设置')).toBeInTheDocument()

    renderPage(<LobbyPage />)
    expect(screen.getByText('大厅列表')).toBeInTheDocument()

    renderPage(<HelpPage />)
    expect(screen.getByText('帮助文档')).toBeInTheDocument()

    renderPage(<LoginPage />)
    expect(screen.getByText('登录')).toBeInTheDocument()

    renderPage(<UserProfilePage />)
    expect(screen.getByText('个人信息')).toBeInTheDocument()
  })
})
```

- [ ] **Step 6: Verify task**

Run:

```bash
cd web-ui
npm run test:unit -- --run
npm run type-check
npm run lint:check
npm run build
```

Expected: all commands pass.

- [ ] **Step 7: Commit**

```bash
git add web-ui dist
git commit -m "feat: rebuild remaining frontend pages"
```

---

### Task 9: Final Visual QA, Dist Reference Check, And Rust Static Tests

**Files:**
- Modify: `dist/index.html`
- Modify: `dist/assets/*`
- Modify: any `web-ui/src/**` files needed for final visual fixes

**Interfaces:**
- Consumes: all previous tasks.
- Produces: verified production frontend assets and a clean worktree.

- [ ] **Step 1: Run full frontend verification**

Run:

```bash
cd web-ui
npm run test:unit -- --run
npm run type-check
npm run lint:check
npm run format:check
npm run build
```

Expected: all commands pass. If `npm run build` emits a known third-party annotation warning but exits 0, record the warning in the final report.

- [ ] **Step 2: Check built asset references**

Run from repo root:

```bash
node -e "const fs=require('fs');const path=require('path');const html=fs.readFileSync('dist/index.html','utf8');const refs=[...html.matchAll(/(?:src|href)=\"([^\"]+)\"/g)].map(m=>m[1]).filter(x=>!x.startsWith('http')&&!x.startsWith('data:'));const missing=refs.filter(ref=>!fs.existsSync(path.join('dist',ref.replace(/^\\//,''))));console.log(JSON.stringify({refs:refs.length,missing},null,2));process.exit(missing.length?1:0)"
```

Expected: JSON output has `"missing": []`.

- [ ] **Step 3: Run Rust static-serving checks**

Run:

```bash
cargo test --locked --test http_tests static_
cargo test --locked --test compat_manifest_tests
```

Expected: both Rust test commands pass.

- [ ] **Step 4: Browser visual review**

Run:

```bash
cd web-ui
npm run dev -- --host 127.0.0.1
```

Open and compare these routes against local screenshots:

```text
http://127.0.0.1:5173/#/dashboard  -> docs/image/dashboard.png
http://127.0.0.1:5173/#/panel      -> docs/image/panel.png
http://127.0.0.1:5173/#/home/clusterIni -> docs/image/home.png
http://127.0.0.1:5173/#/levels/levels   -> docs/image/level.png
http://127.0.0.1:5173/#/mod        -> docs/image/mod1.png
```

Expected: global shell, spacing, card style, menu structure, tabs, and major page regions align with the official screenshots.

- [ ] **Step 5: Commit final verification fixes**

```bash
git add web-ui dist
git commit -m "chore: verify react frontend rebuild"
```

---

## Self-Review

- Spec coverage: Tasks cover scaffold, dependency replacement, API layer, auth/router, Pro shell, Dashboard, Panel, Room, World, Mod, secondary pages, dist refresh, and Rust static tests.
- Scope control: The plan rebuilds a single subsystem, `web-ui`, and keeps Rust API/static boundaries unchanged.
- Dependency consistency: Uses `antd@5.29.3` with `@ant-design/pro-components@2.8.10` to avoid peer conflicts; React Router has an explicit compatibility fallback if DOM exports fail during implementation.
- Test coverage: Every behavioral model/API/helper task starts with tests; route/page smoke and final full verification are required.
