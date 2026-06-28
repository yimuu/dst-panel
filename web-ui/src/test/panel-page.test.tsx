import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { App as AntApp, ConfigProvider } from 'antd'
import type { ReactElement } from 'react'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import PanelPage from '@/pages/PanelPage'
import { routes } from '@/shared/config/routes'

const apiMocks = vi.hoisted(() => ({
  createGameBackup: vi.fn(),
  getAllOnlinePlayers: vi.fn(),
  getLevelServerLog: vi.fn(),
  getLevelStatus: vi.fn(),
  getOnlinePlayers: vi.fn(),
  getSystemInfo: vi.fn(),
  navigate: vi.fn(),
  regenerateWorld: vi.fn(),
  rollbackGame: vi.fn(),
  sendGameCommand: vi.fn(),
  startLevel: vi.fn(),
  stopLevel: vi.fn(),
  updateGame: vi.fn(),
}))

vi.mock('@/features/game/game.api', () => ({
  createGameBackup: apiMocks.createGameBackup,
  getAllOnlinePlayers: apiMocks.getAllOnlinePlayers,
  getLevelLogDownloadUrl: (levelName: string) =>
    `/api/game/level/server/download?levelName=${levelName}&fileName=server_log.txt`,
  getLevelServerLog: apiMocks.getLevelServerLog,
  getLevelStatus: apiMocks.getLevelStatus,
  getOnlinePlayers: apiMocks.getOnlinePlayers,
  getSystemInfo: apiMocks.getSystemInfo,
  regenerateWorld: apiMocks.regenerateWorld,
  rollbackGame: apiMocks.rollbackGame,
  sendGameCommand: apiMocks.sendGameCommand,
  startLevel: apiMocks.startLevel,
  stopLevel: apiMocks.stopLevel,
  updateGame: apiMocks.updateGame,
}))

vi.mock('react-router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-router')>()
  return {
    ...actual,
    useNavigate: () => apiMocks.navigate,
  }
})

function renderWithAntApp(ui: ReactElement) {
  return render(
    <ConfigProvider>
      <AntApp>{ui}</AntApp>
    </ConfigProvider>,
  )
}

async function confirmDangerAction() {
  const confirmButton = await screen.findByRole('button', { name: /确\s*认/ })
  fireEvent.click(confirmButton)
  await waitFor(() => expect(confirmButton).not.toBeInTheDocument())
}

beforeEach(() => {
  vi.clearAllMocks()
  window.localStorage.clear()
  apiMocks.getLevelStatus.mockResolvedValue({
    code: 200,
    msg: 'success',
    data: [
      {
        levelName: 'Master',
        uuid: 'Master',
        status: true,
        is_master: true,
        Ps: { memUage: '128M', cpuUage: '2.1', VSZ: '0', RSS: '0' },
        leveldataoverride: 'return {}',
        modoverrides: 'return {}',
        server_ini: {},
      },
      {
        levelName: 'Caves',
        uuid: 'Caves',
        status: false,
        is_master: false,
        Ps: { memUage: '0M', cpuUage: '0', VSZ: '0', RSS: '0' },
        leveldataoverride: 'return {}',
        modoverrides: 'return {}',
        server_ini: {},
      },
    ],
  })
  apiMocks.getSystemInfo.mockResolvedValue({
    code: 200,
    msg: 'success',
    data: {
      host: {
        os: 'linux',
        hostname: 'dst-host',
        platform: 'ubuntu',
        kernelArch: 'x86_64',
      },
      cpu: {
        cores: 8,
        cpuPercent: [8],
        cpuUsedPercent: 8,
        cpuUsed: 8,
      },
      mem: {
        total: 8000,
        available: 5000,
        used: 3000,
        usedPercent: 37.5,
      },
      disk: {
        devices: [
          {
            device: '/',
            mountpoint: '/',
            fstype: 'apfs',
            opts: 'rw',
            total: 100,
            usage: 12,
            inodesUsage: 0,
          },
        ],
      },
      panelMemUsage: 12_000_000,
      panelCpuUsage: 0.5,
    },
  })
  apiMocks.getLevelServerLog.mockResolvedValue({
    code: 200,
    msg: 'success',
    data: ['[00:00:00]: boot ok'],
  })
  apiMocks.getOnlinePlayers.mockResolvedValue({
    code: 200,
    msg: 'success',
    data: [{ key: '1', day: '12', name: 'Alice', kuId: 'KU_abc', role: 'wilson' }],
  })
  apiMocks.getAllOnlinePlayers.mockResolvedValue({
    code: 200,
    msg: 'success',
    data: [{ key: '2', day: '34', name: 'Bob', kuId: 'KU_def', role: 'wendy' }],
  })
  for (const mock of [
    apiMocks.createGameBackup,
    apiMocks.regenerateWorld,
    apiMocks.rollbackGame,
    apiMocks.sendGameCommand,
    apiMocks.startLevel,
    apiMocks.stopLevel,
    apiMocks.updateGame,
  ]) {
    mock.mockResolvedValue({ code: 200, msg: 'success', data: null })
  }
})

describe('panel page', () => {
  it('loads live resource, level, log, and player data from the backend', async () => {
    renderWithAntApp(<PanelPage />)

    expect(await screen.findByText('dst-host')).toBeInTheDocument()
    expect(screen.getAllByText('Master').length).toBeGreaterThan(0)
    expect(screen.getAllByText('Caves').length).toBeGreaterThan(0)
    expect(await screen.findByText('[00:00:00]: boot ok')).toBeInTheDocument()
    expect(apiMocks.getSystemInfo).toHaveBeenCalledTimes(1)
    expect(apiMocks.getLevelStatus).toHaveBeenCalledTimes(1)
    expect(apiMocks.getLevelServerLog).toHaveBeenCalledWith('Master', 80)
  })

  it('wires panel operation buttons to backend actions and navigation', async () => {
    renderWithAntApp(<PanelPage />)
    await screen.findByText('dst-host')

    fireEvent.click(screen.getByRole('button', { name: /更新游戏/ }))
    fireEvent.click(screen.getByRole('button', { name: /创建备份/ }))
    fireEvent.click(screen.getByRole('button', { name: /地图预览/ }))
    fireEvent.click(screen.getByRole('button', { name: /启动世界/ }))
    fireEvent.click(screen.getByRole('button', { name: /停止世界/ }))
    expect(apiMocks.stopLevel).not.toHaveBeenCalled()
    await confirmDangerAction()

    await waitFor(() => {
      expect(apiMocks.updateGame).toHaveBeenCalledTimes(1)
      expect(apiMocks.createGameBackup).toHaveBeenCalledTimes(1)
      expect(apiMocks.navigate).toHaveBeenCalledWith(routes.genMap)
      expect(apiMocks.startLevel).toHaveBeenCalledWith('Master')
      expect(apiMocks.stopLevel).toHaveBeenCalledWith('Master')
    })
  })

  it('submits console, save, rollback, reset, and player query actions', async () => {
    renderWithAntApp(<PanelPage />)
    await screen.findByText('dst-host')

    fireEvent.change(screen.getByPlaceholderText('输入远程指令'), {
      target: { value: 'c_announce("hi")' },
    })
    fireEvent.keyDown(screen.getByPlaceholderText('输入远程指令'), { key: 'Enter' })
    await confirmDangerAction()
    fireEvent.click(screen.getByRole('button', { name: /保存存档/ }))
    await confirmDangerAction()
    fireEvent.click(screen.getByRole('button', { name: /回档\(3\)天/ }))
    await confirmDangerAction()
    fireEvent.click(screen.getByRole('button', { name: /重置世界/ }))
    await confirmDangerAction()
    fireEvent.click(screen.getByRole('button', { name: /^查\s*询$/ }))

    await waitFor(() => {
      expect(apiMocks.sendGameCommand).toHaveBeenCalledWith({
        levelName: 'Master',
        command: 'c_announce("hi")',
      })
      expect(apiMocks.sendGameCommand).toHaveBeenCalledWith({
        levelName: 'Master',
        command: 'c_save()',
      })
      expect(apiMocks.rollbackGame).toHaveBeenCalledWith(3)
      expect(apiMocks.regenerateWorld).toHaveBeenCalledTimes(1)
      expect(apiMocks.getOnlinePlayers).toHaveBeenCalledWith('Master')
    })
    expect(await screen.findByText('Alice')).toBeInTheDocument()
  })

  it('uses remote tab controls to send command and announcement actions', async () => {
    renderWithAntApp(<PanelPage />)
    await screen.findByText('dst-host')

    fireEvent.click(screen.getByRole('tab', { name: '远程' }))
    fireEvent.change(screen.getByPlaceholderText('输入控制台指令'), {
      target: { value: 'c_reset()' },
    })
    fireEvent.click(screen.getByRole('button', { name: '发送指令' }))
    await confirmDangerAction()
    fireEvent.change(screen.getByPlaceholderText('输入公告内容'), {
      target: { value: '服务器维护' },
    })
    fireEvent.click(screen.getByRole('button', { name: '发送公告' }))
    await confirmDangerAction()

    await waitFor(() => {
      expect(apiMocks.sendGameCommand).toHaveBeenCalledWith({
        levelName: 'Master',
        command: 'c_reset()',
      })
      expect(apiMocks.sendGameCommand).toHaveBeenCalledWith({
        levelName: 'Master',
        command: 'c_announce("服务器维护")',
      })
    })
  })

  it('uses item and custom command tabs to submit backend commands', async () => {
    renderWithAntApp(<PanelPage />)
    await screen.findByText('dst-host')

    fireEvent.click(screen.getByRole('tab', { name: 'TooManyItemsPlus' }))
    fireEvent.change(screen.getByPlaceholderText('请输入物品代码'), {
      target: { value: 'log' },
    })
    fireEvent.click(screen.getByRole('button', { name: '发送物品' }))
    await confirmDangerAction()

    fireEvent.click(screen.getByRole('tab', { name: '自定义指令' }))
    fireEvent.click(screen.getByRole('button', { name: '保存当前世界' }))
    await confirmDangerAction()

    await waitFor(() => {
      expect(apiMocks.sendGameCommand).toHaveBeenCalledWith({
        levelName: 'Master',
        command: 'c_spawn("log", 1)',
      })
      expect(apiMocks.sendGameCommand).toHaveBeenCalledWith({
        levelName: 'Master',
        command: 'c_save()',
      })
    })
  })

  it('saves edited custom commands and exposes them in the custom command tab', async () => {
    renderWithAntApp(<PanelPage />)
    await screen.findByText('dst-host')

    fireEvent.click(screen.getByRole('tab', { name: '自定义指令-编辑' }))
    fireEvent.change(screen.getByPlaceholderText('指令名称'), {
      target: { value: '生成火炬' },
    })
    fireEvent.change(screen.getByPlaceholderText('Lua 指令'), {
      target: { value: 'c_spawn("torch", 1)' },
    })
    fireEvent.click(screen.getByRole('button', { name: '保存指令' }))

    fireEvent.click(screen.getByRole('tab', { name: '自定义指令' }))
    fireEvent.click(screen.getByRole('button', { name: '生成火炬' }))
    await confirmDangerAction()

    await waitFor(() => {
      expect(apiMocks.sendGameCommand).toHaveBeenCalledWith({
        levelName: 'Master',
        command: 'c_spawn("torch", 1)',
      })
    })
  })
})
