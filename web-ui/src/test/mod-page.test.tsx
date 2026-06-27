import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { App as AntApp, ConfigProvider } from 'antd'
import type { ReactElement } from 'react'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import ModPage from '@/pages/ModPage'
import type { ModInfoRecord, ModSearchResponse, UgcModInfo } from '@/features/mods/mod.api'

const apiMocks = vi.hoisted(() => ({
  deleteMod: vi.fn(),
  getMods: vi.fn(),
  getUgcMods: vi.fn(),
  saveModInfo: vi.fn(),
  searchMods: vi.fn(),
  subscribeMod: vi.fn(),
  updateAllModInfo: vi.fn(),
  updateMod: vi.fn(),
  uploadModInfoFile: vi.fn(),
}))

vi.mock('@/features/mods/mod.api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/features/mods/mod.api')>()
  return {
    ...actual,
    deleteMod: apiMocks.deleteMod,
    getMods: apiMocks.getMods,
    getUgcMods: apiMocks.getUgcMods,
    saveModInfo: apiMocks.saveModInfo,
    searchMods: apiMocks.searchMods,
    subscribeMod: apiMocks.subscribeMod,
    updateAllModInfo: apiMocks.updateAllModInfo,
    updateMod: apiMocks.updateMod,
    uploadModInfoFile: apiMocks.uploadModInfoFile,
  }
})

const modList: ModInfoRecord[] = [
  {
    ID: 1,
    auth: 'rezecib, Sarcen',
    consumer_appid: 322330,
    creator_appid: 322330,
    description:
      'By default, shows player arrows when the scoreboard is up, player icons on the minimap globally, and the same for campfires or firepits fueled by charcoal.',
    file_url: '',
    img: '/global.jpg',
    last_time: 1712828023,
    mod_config: [
      {
        name: 'Player Indicators',
        label: 'Player Indicators',
        default: 'scoreboard',
        options: [
          { description: 'Scoreboard', data: 'scoreboard' },
          { description: 'Always', data: 'always' },
        ],
      },
      {
        name: 'Player Icons',
        label: 'Player Icons',
        default: true,
        options: [
          { description: 'Show', data: true },
          { description: 'Hide', data: false },
        ],
      },
    ],
    modid: '378160973',
    name: 'Global Positions',
    update: false,
    v: '1.7.5',
    enabled: true,
  },
  {
    ID: 2,
    auth: '',
    consumer_appid: 322330,
    creator_appid: 322330,
    description: '',
    file_url: '',
    img: '/local.jpg',
    last_time: 0,
    mod_config: [],
    modid: 'workshop-345692228',
    name: '简易血条DST',
    update: false,
    v: '',
    enabled: true,
  },
]

const searchResult: ModSearchResponse = {
  page: 1,
  size: 10,
  total: 1,
  totalPage: 1,
  data: [
    {
      author: '76561198025931302',
      created: 1712828023,
      img: '/global.jpg',
      modid: '378160973',
      name: 'Global Positions',
      score: 5,
      subscription: '7.85m',
      time: '2024-04-11',
    },
  ],
}

function renderWithAntApp(ui: ReactElement) {
  return render(
    <ConfigProvider>
      <AntApp>{ui}</AntApp>
    </ConfigProvider>,
  )
}

beforeEach(() => {
  vi.clearAllMocks()
  apiMocks.getMods.mockResolvedValue({ code: 200, msg: 'success', data: modList })
  apiMocks.getUgcMods.mockResolvedValue({
    code: 200,
    msg: 'success',
    data: [
      {
        img: '/global.jpg',
        name: 'Global Positions',
        timelast: 1712828023,
        timeupdated: 1712828023,
        workshopId: '378160973',
      } satisfies UgcModInfo,
    ],
  })
  apiMocks.searchMods.mockResolvedValue({ code: 200, msg: 'success', data: searchResult })
  apiMocks.saveModInfo.mockResolvedValue({ code: 200, msg: 'success', data: modList[0] })
  apiMocks.subscribeMod.mockResolvedValue({ code: 200, msg: 'success', data: modList[0] })
  apiMocks.updateAllModInfo.mockResolvedValue({ code: 200, msg: 'success', data: null })
  apiMocks.updateMod.mockResolvedValue({ code: 200, msg: 'success', data: modList[0] })
  apiMocks.deleteMod.mockResolvedValue({ code: 200, msg: 'success', data: null })
  apiMocks.uploadModInfoFile.mockResolvedValue({ code: 200, msg: 'success', data: null })
})

describe('mod page', () => {
  it('renders the official mod settings workflow and calls core actions', async () => {
    renderWithAntApp(<ModPage />)

    expect((await screen.findAllByText('Global Positions')).length).toBeGreaterThan(0)
    expect(screen.getByRole('tab', { name: '模组设置' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: '模组订阅' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Ugc模组' })).toBeInTheDocument()
    expect(screen.getByText(/请先启动世界/)).toBeInTheDocument()
    expect(screen.getAllByRole('button', { name: /保\s*存/ }).length).toBeGreaterThan(0)
    expect(screen.getByRole('button', { name: /全部更新/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /上传自定义模组配置/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /保存到森林/ })).toBeInTheDocument()
    expect(screen.getByText('版本: 1.7.5')).toBeInTheDocument()
    expect(screen.getByText('创意工坊:378160973')).toBeInTheDocument()
    expect(screen.getByText('饥荒联机版兼容')).toBeInTheDocument()
    expect(screen.getByText(/By default, shows player arrows/)).toBeInTheDocument()

    const globalRow = screen.getByTestId('mod-row-378160973')
    fireEvent.click(within(globalRow).getByRole('button', { name: /删除/ }))
    await waitFor(() => {
      expect(apiMocks.deleteMod).toHaveBeenCalledWith('378160973')
    })

    fireEvent.click(screen.getByRole('button', { name: /全部更新/ }))
    await waitFor(() => {
      expect(apiMocks.updateAllModInfo).toHaveBeenCalledTimes(1)
    })
  })

  it('renders subscription search and subscribe action', async () => {
    renderWithAntApp(<ModPage />)

    fireEvent.click(screen.getByRole('tab', { name: '模组订阅' }))
    expect(screen.getByPlaceholderText('输入创意工坊 ID 或关键词')).toBeInTheDocument()
    fireEvent.change(screen.getByPlaceholderText('输入创意工坊 ID 或关键词'), {
      target: { value: 'global positions' },
    })
    fireEvent.click(screen.getByRole('button', { name: /搜索/ }))

    expect(await screen.findByText('订阅: 7.85m')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: /订\s*阅/ }))
    await waitFor(() => {
      expect(apiMocks.subscribeMod).toHaveBeenCalledWith('378160973')
    })
  })

  it('opens the option drawer with parsed mod config options', async () => {
    renderWithAntApp(<ModPage />)

    expect((await screen.findAllByText('Global Positions')).length).toBeGreaterThan(0)
    fireEvent.click(screen.getByRole('button', { name: /选\s*项/ }))

    expect(await screen.findByText('Player Indicators')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /保存偏好/ })).toBeInTheDocument()
  })
})
