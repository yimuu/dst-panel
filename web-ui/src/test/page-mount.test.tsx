import { render, screen } from '@testing-library/react'
import { App as AntApp, ConfigProvider } from 'antd'
import type { ReactElement } from 'react'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import BackupPage from '@/pages/BackupPage'
import HelpPage from '@/pages/HelpPage'
import InitPage from '@/pages/InitPage'
import LobbyPage from '@/pages/LobbyPage'
import LoginPage from '@/pages/LoginPage'
import PlayerLogPage from '@/pages/PlayerLogPage'
import SettingsPage from '@/pages/SettingsPage'
import UserProfilePage from '@/pages/UserProfilePage'

const apiMocks = vi.hoisted(() => ({
  getBackups: vi.fn(),
  getDstConfig: vi.fn(),
  getPlayerLogs: vi.fn(),
  getCurrentUser: vi.fn(),
}))

vi.mock('@/features/backups/backup.api', () => ({
  createBackup: vi.fn(),
  deleteBackups: vi.fn(),
  getBackups: apiMocks.getBackups,
  getBackupDownloadUrl: (fileName: string) => `/api/game/backup/download?fileName=${fileName}`,
  restoreBackup: vi.fn(),
  uploadBackup: vi.fn(),
}))

vi.mock('@/features/settings/settings.api', () => ({
  getDstConfig: apiMocks.getDstConfig,
  saveDstConfig: vi.fn(),
}))

vi.mock('@/features/player-logs/player-log.api', () => ({
  deletePlayerLogs: vi.fn(),
  getPlayerLogs: apiMocks.getPlayerLogs,
}))

vi.mock('@/features/auth/auth.api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/features/auth/auth.api')>()
  return {
    ...actual,
    getCurrentUser: apiMocks.getCurrentUser,
  }
})

vi.mock('react-router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-router')>()
  return {
    ...actual,
    useNavigate: () => vi.fn(),
  }
})

function renderWithAntApp(ui: ReactElement) {
  return render(
    <ConfigProvider>
      <AntApp>{ui}</AntApp>
    </ConfigProvider>,
  )
}

beforeEach(() => {
  vi.clearAllMocks()
  apiMocks.getBackups.mockResolvedValue({
    code: 200,
    msg: 'success',
    data: [
      {
        createTime: '2024-04-11T09:33:43Z',
        fileName: 'Cluster1_001.zip',
        fileSize: 1048576,
        time: 1712828023,
      },
    ],
  })
  apiMocks.getDstConfig.mockResolvedValue({
    code: 200,
    msg: 'success',
    data: {
      steamcmd: '/opt/steamcmd',
      force_install_dir: '/opt/dst',
      backup: '/data/backup',
      mod_download_path: '/data/mods',
      cluster: 'Cluster1',
      persistent_storage_root: '/data/klei',
      conf_dir: 'DoNotStarveTogether',
      ugc_directory: '',
      donot_starve_server_directory: '',
      bin: 32,
      beta: 0,
    },
  })
  apiMocks.getPlayerLogs.mockResolvedValue({
    code: 200,
    msg: 'success',
    data: {
      data: [
        {
          ID: 1,
          name: 'Wilson',
          kuId: 'KU_ABC',
          role: '威尔逊',
          action: '进入游戏',
          ip: '127.0.0.1',
          createdAt: '2024-04-11T09:33:43Z',
        },
      ],
      page: 1,
      size: 10,
      total: 1,
      totalPages: 1,
    },
  })
  apiMocks.getCurrentUser.mockResolvedValue({
    code: 200,
    msg: 'success',
    data: {
      username: 'admin',
      displayName: '管理员',
      photoURL: '',
    },
  })
})

describe('remaining pages', () => {
  it('renders backup, player log, and settings pages with backend data', async () => {
    renderWithAntApp(<BackupPage />)
    expect(await screen.findByText('Cluster1_001.zip')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /创建备份/ })).toBeInTheDocument()

    renderWithAntApp(<PlayerLogPage />)
    expect(await screen.findByText('Wilson')).toBeInTheDocument()
    expect(screen.getByText('进入游戏')).toBeInTheDocument()

    renderWithAntApp(<SettingsPage />)
    expect(await screen.findByDisplayValue('/opt/steamcmd')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /保存设置/ })).toBeInTheDocument()
  })

  it('renders static utility pages and auth pages', async () => {
    renderWithAntApp(<LobbyPage />)
    expect(screen.getByText('大厅列表')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /搜索大厅/ })).toBeInTheDocument()

    renderWithAntApp(<HelpPage />)
    expect(screen.getByText('帮助文档')).toBeInTheDocument()
    expect(screen.getByText('Docker Compose')).toBeInTheDocument()

    renderWithAntApp(<LoginPage />)
    expect(screen.getByRole('button', { name: /登\s*录/ })).toBeInTheDocument()

    renderWithAntApp(<InitPage />)
    expect(screen.getByRole('button', { name: /初\s*始化/ })).toBeInTheDocument()

    renderWithAntApp(<UserProfilePage />)
    expect(await screen.findByText('管理员')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /保存资料/ })).toBeInTheDocument()
  })
})
