import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { App as AntApp, ConfigProvider } from 'antd'
import type { ReactElement } from 'react'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import ClusterIniPage from '@/pages/ClusterIniPage'
import PlayerListPage from '@/pages/PlayerListPage'
import type { ClusterIniEnvelope } from '@/features/room/room.api'

const apiMocks = vi.hoisted(() => ({
  getClusterIni: vi.fn(),
  saveClusterIni: vi.fn(),
  getPlayerList: vi.fn(),
  savePlayerList: vi.fn(),
  addPlayerListEntries: vi.fn(),
  removePlayerListEntries: vi.fn(),
}))

vi.mock('@/features/room/room.api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/features/room/room.api')>()
  return {
    ...actual,
    getClusterIni: apiMocks.getClusterIni,
    saveClusterIni: apiMocks.saveClusterIni,
    getPlayerList: apiMocks.getPlayerList,
    savePlayerList: apiMocks.savePlayerList,
    addPlayerListEntries: apiMocks.addPlayerListEntries,
    removePlayerListEntries: apiMocks.removePlayerListEntries,
  }
})

const clusterEnvelope: ClusterIniEnvelope = {
  cluster: {
    game_mode: 'survival',
    max_players: 12,
    pvp: false,
    pause_when_nobody: true,
    vote_enabled: true,
    vote_kick_enabled: true,
    lan_only_cluster: false,
    cluster_intention: 'cooperative',
    cluster_description: '来自后端',
    cluster_password: '',
    cluster_name: '后端房间',
    offline_cluster: false,
    cluster_language: 'zh',
    whitelist_slots: 3,
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
  token: 'backend-token',
}

function renderWithAntApp(ui: ReactElement) {
  return render(
    <ConfigProvider>
      <AntApp>{ui}</AntApp>
    </ConfigProvider>,
  )
}

async function confirmPopconfirm() {
  fireEvent.click(await screen.findByRole('button', { name: /确\s*认/ }))
}

beforeEach(() => {
  vi.clearAllMocks()
  apiMocks.getClusterIni.mockResolvedValue({
    code: 200,
    msg: 'success',
    data: clusterEnvelope,
  })
  apiMocks.saveClusterIni.mockResolvedValue({ code: 200, msg: 'success', data: clusterEnvelope })
  apiMocks.getPlayerList.mockResolvedValue({ code: 200, msg: 'success', data: ['KU_EXISTING'] })
  apiMocks.savePlayerList.mockResolvedValue({ code: 200, msg: 'success', data: null })
  apiMocks.addPlayerListEntries.mockResolvedValue({ code: 200, msg: 'success', data: null })
  apiMocks.removePlayerListEntries.mockResolvedValue({ code: 200, msg: 'success', data: null })
})

describe('room setting page', () => {
  it('loads cluster.ini from backend and saves edited values', async () => {
    renderWithAntApp(<ClusterIniPage />)

    const nameInput = await screen.findByDisplayValue('后端房间')
    expect(apiMocks.getClusterIni).toHaveBeenCalledTimes(1)
    expect(screen.queryByText('emoji')).not.toBeInTheDocument()
    expect(screen.queryByText('-', { exact: true })).not.toBeInTheDocument()

    fireEvent.change(nameInput, { target: { value: '更新房间' } })
    fireEvent.click(screen.getByRole('button', { name: /保\s*存/ }))

    await waitFor(() => {
      expect(apiMocks.saveClusterIni).toHaveBeenCalledWith({
        ...clusterEnvelope,
        cluster: {
          ...clusterEnvelope.cluster,
          cluster_name: '更新房间',
        },
      })
    })
  })
})

describe('player list page', () => {
  it('loads, refreshes, adds, and removes adminlist through backend APIs', async () => {
    renderWithAntApp(<PlayerListPage kind="adminlist" />)

    expect(await screen.findByText('KU_EXISTING')).toBeInTheDocument()
    expect(apiMocks.getPlayerList).toHaveBeenCalledWith('adminlist')

    fireEvent.click(screen.getByRole('button', { name: /刷新/ }))
    await waitFor(() => {
      expect(apiMocks.getPlayerList).toHaveBeenCalledTimes(2)
    })

    fireEvent.change(screen.getByPlaceholderText('输入 KU ID'), {
      target: { value: 'KU_NEW' },
    })
    fireEvent.click(screen.getByRole('button', { name: /添加/ }))
    await waitFor(() => {
      expect(apiMocks.addPlayerListEntries).toHaveBeenCalledWith('adminlist', ['KU_NEW'])
    })
    expect(await screen.findByText('KU_NEW')).toBeInTheDocument()
    const kuIds = screen
      .getAllByRole('row')
      .map((row) => within(row).queryAllByRole('cell')[0]?.textContent)
      .filter(Boolean)
    expect(kuIds).toEqual(['KU_EXISTING', 'KU_NEW'])

    const row = screen.getByText('KU_EXISTING').closest('tr')
    expect(row).not.toBeNull()
    fireEvent.click(within(row as HTMLTableRowElement).getByRole('button', { name: /删除/ }))
    await confirmPopconfirm()
    await waitFor(() => {
      expect(apiMocks.removePlayerListEntries).toHaveBeenCalledWith('adminlist', ['KU_EXISTING'])
    })
  })

  it('uses savePlayerList for whitelist mutations because the backend has no append/delete route', async () => {
    renderWithAntApp(<PlayerListPage kind="whitelist" />)

    expect(await screen.findByText('KU_EXISTING')).toBeInTheDocument()
    fireEvent.change(screen.getByPlaceholderText('输入 KU ID'), {
      target: { value: 'KU_WHITE' },
    })
    fireEvent.click(screen.getByRole('button', { name: /添加/ }))

    await waitFor(() => {
      expect(apiMocks.savePlayerList).toHaveBeenCalledWith('whitelist', ['KU_WHITE', 'KU_EXISTING'])
    })

    const row = screen.getByText('KU_EXISTING').closest('tr')
    expect(row).not.toBeNull()
    fireEvent.click(within(row as HTMLTableRowElement).getByRole('button', { name: /删除/ }))
    await confirmPopconfirm()
    await waitFor(() => {
      expect(apiMocks.savePlayerList).toHaveBeenLastCalledWith('whitelist', ['KU_WHITE'])
    })
  })
})
