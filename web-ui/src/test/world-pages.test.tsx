import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { App as AntApp, ConfigProvider } from 'antd'
import type { ReactElement } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import MapPreviewPage from '@/pages/MapPreviewPage'
import PreinstallPage from '@/pages/PreinstallPage'
import WorldLevelsPage from '@/pages/WorldLevelsPage'
import WorldModSelectionPage from '@/pages/WorldModSelectionPage'
import type { WorldSettingsDefinition } from '@/features/worlds/world-settings-model'

const apiMocks = vi.hoisted(() => ({
  getLevels: vi.fn(),
  getWorldSettingsDefinition: vi.fn(),
  generateMap: vi.fn(),
  applyPreinstall: vi.fn(),
}))

vi.mock('@/features/levels/level.api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/features/levels/level.api')>()
  return {
    ...actual,
    getLevels: apiMocks.getLevels,
  }
})

vi.mock('@/features/maps/map.api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/features/maps/map.api')>()
  return {
    ...actual,
    getWorldSettingsDefinition: apiMocks.getWorldSettingsDefinition,
    generateMap: apiMocks.generateMap,
    getMapImageUrl: (levelName: string) => `/map/${levelName}.png`,
  }
})

vi.mock('@/features/maps/map-state', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/features/maps/map-state')>()
  return {
    ...actual,
    applyPreinstall: apiMocks.applyPreinstall,
  }
})

const worldSettings: WorldSettingsDefinition = {
  zh: {
    forest: {
      WORLDSETTINGS_GROUP: {
        global: {
          order: 1,
          text: '全局',
          atlas: {
            name: 'worldsettings_customization',
            width: 2048,
            height: 1024,
            item_size: 128,
          },
          desc: { default: '默认', long: '长' },
          items: {
            autumn: {
              text: '秋',
              value: 'default',
              image: { x: 0.25, y: 0.125 },
            },
          },
        },
      },
      WORLDGEN_GROUP: {
        resources: {
          order: 1,
          text: '资源',
          atlas: {
            name: 'worldgen_customization',
            width: 2048,
            height: 1024,
            item_size: 128,
          },
          desc: { default: '默认', often: '较多' },
          items: {
            grass: {
              text: '草',
              value: 'often',
              image: { x: 0.125, y: 0 },
            },
          },
        },
      },
    },
  },
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
  apiMocks.getLevels.mockResolvedValue({
    code: 200,
    msg: 'success',
    data: [
      {
        levelName: '森林',
        uuid: 'Master',
        serverini: '[NETWORK]',
        leveldataoverride: 'return {}',
        modoverrides: 'return {}',
      },
      {
        levelName: '洞穴',
        uuid: 'Caves',
        serverini: '[NETWORK]',
        leveldataoverride: 'return {}',
        modoverrides: 'return {}',
      },
    ],
  })
  apiMocks.getWorldSettingsDefinition.mockResolvedValue(worldSettings)
  apiMocks.generateMap.mockResolvedValue({ code: 200, msg: 'success', data: null })
  apiMocks.applyPreinstall.mockResolvedValue({ code: 200, msg: 'success', data: null })
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => ({
      ok: true,
      json: async () => [
        {
          name: '标准世界',
          description: '森林和洞穴',
          value: 'standard',
          src: '/template.svg',
        },
      ],
    })),
  )
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('world task pages', () => {
  it('renders level tabs and world setting controls from backend data', async () => {
    renderWithAntApp(<WorldLevelsPage />)

    expect(await screen.findByText('森林')).toBeInTheDocument()
    expect(screen.getByText('洞穴')).toBeInTheDocument()
    expect(screen.getByText('世界生成')).toBeInTheDocument()
    expect(screen.getByText('秋')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('tab', { name: '世界生成' }))
    expect(screen.getByText('草')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /保\s*存/ })).toBeInTheDocument()
    expect(apiMocks.getLevels).toHaveBeenCalledTimes(1)
    expect(apiMocks.getWorldSettingsDefinition).toHaveBeenCalledTimes(1)
  })

  it('renders selector mod configuration actions', () => {
    renderWithAntApp(<WorldModSelectionPage />)

    expect(screen.getByText('多层选择器')).toBeInTheDocument()
    expect(screen.getByText('世界配置同步')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '设置默认多层选择器' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: '保存配置' })).toBeInTheDocument()
  })

  it('renders preinstall templates from public json', async () => {
    renderWithAntApp(<PreinstallPage />)

    expect(await screen.findByText('标准世界')).toBeInTheDocument()
    expect(screen.getByText('森林和洞穴')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: '应用模板' }))

    await waitFor(() => {
      expect(apiMocks.applyPreinstall).toHaveBeenCalledWith('standard')
    })
  })

  it('renders map preview image and generate action', async () => {
    renderWithAntApp(<MapPreviewPage />)

    expect(screen.getByText('预览地图')).toBeInTheDocument()
    expect(screen.getByRole('img', { name: 'Master 地图预览' })).toHaveAttribute(
      'src',
      '/map/Master.png',
    )
    fireEvent.click(screen.getByRole('button', { name: '生成地图' }))

    await waitFor(() => {
      expect(apiMocks.generateMap).toHaveBeenCalledWith('Master')
    })
  })
})
