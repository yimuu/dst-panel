import { flushPromises, mount, type DOMWrapper, type VueWrapper } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import * as modApi from '@/features/mods/mod.api'
import * as settingsApi from '@/features/settings/settings.api'
import WorldModSelectionPage from '@/pages/WorldModSelectionPage.vue'
import type { ApiEnvelope } from '@/shared/api/types'
import type { GameConfig, ModSummary } from '@/shared/types/domain'

vi.mock('element-plus', async () => {
  const actual = await vi.importActual<typeof import('element-plus')>('element-plus')

  return {
    ...actual,
    ElMessage: {
      success: vi.fn(),
      error: vi.fn(),
    },
  }
})

vi.mock('@/features/mods/mod.api', () => ({
  listMods: vi.fn(),
}))

vi.mock('@/features/settings/settings.api', () => ({
  getGameConfig: vi.fn(),
  saveGameConfig: vi.fn(),
}))

const listMods = vi.mocked(modApi.listMods)
const getGameConfig = vi.mocked(settingsApi.getGameConfig)
const saveGameConfig = vi.mocked(settingsApi.saveGameConfig)

let wrapper: VueWrapper | undefined

function success<T>(data: T): ApiEnvelope<T> {
  return {
    code: 0,
    data,
  }
}

function createGameConfig(): GameConfig {
  return {
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
  }
}

function mountWorldModSelectionPage(): VueWrapper {
  wrapper = mount(WorldModSelectionPage, {
    attachTo: document.body,
    global: {
      plugins: [ElementPlus],
    },
  })

  return wrapper
}

function findButton(label: string): DOMWrapper<HTMLButtonElement> {
  const button = wrapper
    ?.findAll<HTMLButtonElement>('button')
    .find((candidate) => candidate.text().includes(label))

  if (!button) {
    throw new Error(`未找到按钮：${label}`)
  }

  return button
}

describe('world mod selection page', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    listMods.mockResolvedValue(
      success<ModSummary[]>([
        {
          modid: '123',
          name: '几何布局',
          description: '建筑辅助',
        },
      ]),
    )
    getGameConfig.mockResolvedValue(success(createGameConfig()))
    saveGameConfig.mockResolvedValue(success(null))
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
    document.body.innerHTML = ''
  })

  it('loads installed mods and saves selected mods into game config modData', async () => {
    mountWorldModSelectionPage()
    await flushPromises()

    expect(listMods).toHaveBeenCalled()
    expect(getGameConfig).toHaveBeenCalled()
    expect(wrapper?.text()).toContain('选择模组')

    await wrapper?.find('[data-test="mod-toggle-123"]').trigger('click')
    await findButton('保存选择').trigger('click')
    await flushPromises()

    expect(saveGameConfig).toHaveBeenCalledWith(
      expect.objectContaining({
        modData: expect.stringContaining('workshop-123'),
      }),
    )
  })
})
