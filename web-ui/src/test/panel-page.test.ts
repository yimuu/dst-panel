import { flushPromises, mount } from '@vue/test-utils'
import ElementPlus, { ElMessage } from 'element-plus'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import * as gameApi from '@/features/game/game.api'
import * as levelApi from '@/features/levels/level.api'
import PanelPage from '@/pages/PanelPage.vue'
import type { ApiEnvelope } from '@/shared/api/types'
import { useClusterStore } from '@/shared/stores/cluster'
import type { LevelSummary } from '@/shared/types/domain'

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

vi.mock('@/features/levels/level.api', () => ({
  listLevels: vi.fn(),
}))

vi.mock('@/features/game/game.api', () => ({
  getGameStatus: vi.fn(),
  startLevel: vi.fn(),
  stopLevel: vi.fn(),
}))

const listLevels = vi.mocked(levelApi.listLevels)
const getGameStatus = vi.mocked(gameApi.getGameStatus)
const startLevel = vi.mocked(gameApi.startLevel)
const stopLevel = vi.mocked(gameApi.stopLevel)
const successMessage = vi.mocked(ElMessage.success)

function success<T>(data: T): ApiEnvelope<T> {
  return {
    code: 0,
    data,
  }
}

function mountPanelPage() {
  const pinia = createPinia()
  setActivePinia(pinia)
  useClusterStore().setSelectedCluster('Cluster_1')

  return mount(PanelPage, {
    global: {
      plugins: [pinia, ElementPlus],
    },
  })
}

describe('panel page runtime operations', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    listLevels.mockResolvedValue(success([]))
    getGameStatus.mockResolvedValue(success([]))
    startLevel.mockResolvedValue(success(null))
    stopLevel.mockResolvedValue(success(null))
  })

  it('loads runtime level status instead of config levels on mount', async () => {
    const configLevels: LevelSummary[] = [
      {
        uuid: 'ConfigShard',
        levelName: '配置世界',
        status: false,
      },
    ]
    const runtimeLevels: LevelSummary[] = [
      {
        uuid: 'Master',
        levelName: '森林',
        is_master: true,
        status: true,
      },
    ]
    listLevels.mockResolvedValue(success(configLevels))
    getGameStatus.mockResolvedValue(success(runtimeLevels))

    const wrapper = mountPanelPage()
    await flushPromises()

    expect(getGameStatus).toHaveBeenCalledWith('Cluster_1')
    expect(listLevels).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('森林')
    expect(wrapper.text()).not.toContain('配置世界')
  })

  it('restarts a runtime level using shard uuid before display level name', async () => {
    const runtimeLevels: LevelSummary[] = [
      {
        uuid: 'Master',
        levelName: '森林',
        is_master: true,
        status: true,
      },
    ]
    listLevels.mockResolvedValue(success(runtimeLevels))
    getGameStatus.mockResolvedValue(success(runtimeLevels))

    const wrapper = mountPanelPage()
    await flushPromises()

    const restartButton = wrapper.findAll('button').find((button) => button.text().includes('重启'))

    expect(restartButton?.exists()).toBe(true)

    await restartButton?.trigger('click')
    await flushPromises()

    expect(stopLevel).toHaveBeenCalledWith('Master', 'Cluster_1')
    expect(startLevel).toHaveBeenCalledWith('Master', 'Cluster_1')
    expect(stopLevel.mock.invocationCallOrder[0] ?? Number.POSITIVE_INFINITY).toBeLessThan(
      startLevel.mock.invocationCallOrder[0] ?? 0,
    )
    expect(successMessage).toHaveBeenCalledWith('操作已提交')
  })
})
