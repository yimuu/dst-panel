import type { Component } from 'vue'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createPinia } from 'pinia'
import { describe, expect, it, vi } from 'vitest'

import BackupPage from '@/pages/BackupPage.vue'
import ClusterIniPage from '@/pages/ClusterIniPage.vue'
import DashboardPage from '@/pages/DashboardPage.vue'
import HelpPage from '@/pages/HelpPage.vue'
import InitPage from '@/pages/InitPage.vue'
import LobbyPage from '@/pages/LobbyPage.vue'
import LoginPage from '@/pages/LoginPage.vue'
import ModPage from '@/pages/ModPage.vue'
import PanelPage from '@/pages/PanelPage.vue'
import PlayerListPage from '@/pages/PlayerListPage.vue'
import PlayerLogPage from '@/pages/PlayerLogPage.vue'
import PreinstallPage from '@/pages/PreinstallPage.vue'
import SettingsPage from '@/pages/SettingsPage.vue'
import UserProfilePage from '@/pages/UserProfilePage.vue'
import WorldLevelsPage from '@/pages/WorldLevelsPage.vue'
import WorldModSelectionPage from '@/pages/WorldModSelectionPage.vue'
import MapPreviewPage from '@/pages/MapPreviewPage.vue'

vi.mock('@/features/backups/backup.api', () => ({
  listBackups: vi.fn(async () => ({
    code: 0,
    data: [],
  })),
  createBackup: vi.fn(async () => ({
    code: 0,
    data: null,
  })),
  restoreBackup: vi.fn(async () => ({
    code: 0,
    data: null,
  })),
  deleteBackups: vi.fn(async () => ({
    code: 0,
    data: null,
  })),
}))

vi.mock('@/features/levels/level.api', () => ({
  listLevels: vi.fn(async () => ({
    code: 0,
    data: [],
  })),
  createLevel: vi.fn(async () => ({
    code: 0,
    data: null,
  })),
  saveLevels: vi.fn(async () => ({
    code: 0,
    data: null,
  })),
  deleteLevel: vi.fn(async () => ({
    code: 0,
    data: null,
  })),
}))

vi.mock('@/features/game/game.api', () => ({
  getGameStatus: vi.fn(async () => ({
    code: 0,
    data: [],
  })),
  startLevel: vi.fn(async () => ({
    code: 0,
    data: null,
  })),
  stopLevel: vi.fn(async () => ({
    code: 0,
    data: null,
  })),
  applyPreinstallTemplate: vi.fn(async () => ({
    code: 0,
    data: null,
  })),
}))

vi.mock('@/features/room/room.api', () => ({
  getClusterIni: vi.fn(async () => ({
    code: 0,
    data: {
      cluster: {
        game_mode: 'survival',
        max_players: 8,
        pvp: false,
        pause_when_nobody: true,
        vote_enabled: true,
        vote_kick_enabled: true,
        lan_only_cluster: false,
        cluster_intention: 'cooperative',
        cluster_description: '',
        cluster_password: '',
        cluster_name: '测试世界',
        offline_cluster: false,
        cluster_language: 'zh',
        whitelist_slots: 0,
        tick_rate: 15,
        console_enabled: true,
        max_snapshots: 6,
        shard_enabled: true,
        bind_ip: '0.0.0.0',
        master_ip: '127.0.0.1',
        master_port: 10888,
        cluster_key: '',
        steam_group_id: '',
        steam_group_only: false,
        steam_group_admins: false,
      },
      token: 'server-token',
    },
  })),
  saveClusterIni: vi.fn(async () => ({
    code: 0,
    data: null,
  })),
  getPlayerList: vi.fn(async () => ({
    code: 0,
    data: [],
  })),
  savePlayerList: vi.fn(async () => ({
    code: 0,
    data: null,
  })),
}))

vi.mock('@/features/mods/mod.api', () => ({
  listMods: vi.fn(async () => ({
    code: 0,
    data: [],
  })),
  searchMods: vi.fn(async () => ({
    code: 0,
    data: {
      data: [],
    },
  })),
  saveModInfo: vi.fn(async () => ({
    code: 0,
    data: null,
  })),
}))

vi.mock('@/features/maps/map.api', async () => {
  const actual = await vi.importActual<typeof import('@/features/maps/map.api')>(
    '@/features/maps/map.api',
  )

  return {
    ...actual,
    generateMap: vi.fn(async () => ({
      code: 0,
      data: null,
    })),
    checkWalrusHutPlains: vi.fn(async () => ({
      code: 0,
      data: false,
    })),
    getSessionFile: vi.fn(async () => ({
      code: 0,
      data: '',
    })),
  }
})

vi.mock('@/features/settings/settings.api', () => ({
  getDstConfig: vi.fn(async () => ({
    code: 0,
    data: {
      steamcmd: '/opt/steamcmd',
      force_install_dir: '/srv/dst',
      donot_starve_server_directory: '',
      cluster: 'Cluster_1',
      backup: '/srv/backup',
      mod_download_path: '/srv/mods',
      bin: 64,
      beta: 0,
      ugc_directory: '',
      persistent_storage_root: '/srv/klei',
      conf_dir: 'DoNotStarveTogether',
    },
  })),
  saveDstConfig: vi.fn(async () => ({
    code: 0,
    data: null,
  })),
  getLobbyServerDetail: vi.fn(async () => ({
    code: 0,
    data: {},
  })),
  getGameConfig: vi.fn(async () => ({
    code: 0,
    data: {
      clusterIntention: 'cooperative',
      clusterName: '测试世界',
      clusterDescription: '',
      gameMode: 'survival',
      pvp: false,
      maxPlayers: 8,
      max_snapshots: 6,
      clusterPassword: '',
      token: 'server-token',
      masterMapData: '',
      cavesMapData: '',
      modData: '',
      type: 0,
      pause_when_nobody: true,
      vote_enabled: true,
    },
  })),
  saveGameConfig: vi.fn(async () => ({
    code: 0,
    data: null,
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

const routePages: Array<[string, Component, Record<string, unknown>?]> = [
  ['登录页', LoginPage],
  ['初始化页', InitPage],
  ['仪表盘页', DashboardPage],
  ['面板页', PanelPage],
  ['集群设置页', ClusterIniPage],
  [
    '玩家列表页',
    PlayerListPage,
    {
      kind: 'adminlist',
      title: '管理员列表',
      description: '管理员列表维护',
    },
  ],
  ['世界页', WorldLevelsPage],
  ['选择模组页', WorldModSelectionPage],
  ['预设模板页', PreinstallPage],
  ['地图预览页', MapPreviewPage],
  ['模组页', ModPage],
  ['备份页', BackupPage],
  ['设置页', SettingsPage],
  ['玩家日志页', PlayerLogPage],
  ['大厅页', LobbyPage],
  ['帮助页', HelpPage],
  ['个人信息页', UserProfilePage],
]

describe('route page skeletons', () => {
  it.each(routePages)('%s can mount', (_name, component, props = {}) => {
    const wrapper = mount(component, {
      props,
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
