import type { Component } from 'vue'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createPinia } from 'pinia'
import { describe, expect, it, vi } from 'vitest'

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

vi.mock('@/features/levels/level.api', () => ({
  listLevels: vi.fn(async () => ({
    code: 0,
    data: [],
  })),
}))

vi.mock('vue-router', async () => {
  const actual = await vi.importActual<typeof import('vue-router')>('vue-router')

  return {
    ...actual,
    useRoute: () => ({
      query: {},
    }),
    useRouter: () => ({
      replace: vi.fn(),
      push: vi.fn(),
    }),
  }
})

const routePages: Array<[string, Component]> = [
  ['登录页', LoginPage],
  ['初始化页', InitPage],
  ['仪表盘页', DashboardPage],
  ['面板页', PanelPage],
  ['世界页', WorldLevelsPage],
  ['模组页', ModPage],
  ['备份页', BackupPage],
  ['设置页', SettingsPage],
  ['玩家日志页', PlayerLogPage],
  ['大厅页', LobbyPage],
  ['帮助页', HelpPage],
  ['个人信息页', UserProfilePage],
]

describe('route page skeletons', () => {
  it.each(routePages)('%s can mount', (_name, component) => {
    const wrapper = mount(component, {
      global: {
        plugins: [createPinia(), ElementPlus],
        stubs: {
          RouterLink: true,
          RouterView: true,
        },
      },
    })

    expect(wrapper.exists()).toBe(true)
  })
})
