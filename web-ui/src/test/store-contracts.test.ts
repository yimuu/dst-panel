import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import * as clusterApi from '@/features/clusters/cluster.api'
import * as gameApi from '@/features/game/game.api'
import * as levelApi from '@/features/levels/level.api'
import type { ApiEnvelope } from '@/shared/api/types'
import { useClusterStore } from '@/shared/stores/cluster'
import { useLevelStore } from '@/shared/stores/levels'
import { useThemeStore } from '@/shared/stores/theme'
import type { ClusterSummary, LevelSummary } from '@/shared/types/domain'

vi.mock('@/features/clusters/cluster.api', () => ({
  listClusters: vi.fn(),
}))

vi.mock('@/features/levels/level.api', () => ({
  listLevels: vi.fn(),
}))

vi.mock('@/features/game/game.api', () => ({
  getGameStatus: vi.fn(),
}))

const listClusters = vi.mocked(clusterApi.listClusters)
const listLevels = vi.mocked(levelApi.listLevels)
const getGameStatus = vi.mocked(gameApi.getGameStatus)

function success<T>(data: T): ApiEnvelope<T> {
  return {
    code: 0,
    data,
  }
}

describe('store contracts', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('exports the singular level store name used by route code', () => {
    const store = useLevelStore()

    expect(store.levels).toEqual([])
    expect(store.loading).toBe(false)
    expect(store.runtimeLevels).toEqual([])
    expect(store.runtimeLoading).toBe(false)
  })

  it('refreshes runtime levels from game status without reading config levels', async () => {
    const runtimeLevels: LevelSummary[] = [
      {
        uuid: 'Master',
        levelName: '森林',
        is_master: true,
        status: true,
      },
    ]
    getGameStatus.mockResolvedValue(success(runtimeLevels))

    const store = useLevelStore()

    await expect(store.refreshRuntimeLevels()).resolves.toEqual(runtimeLevels)

    expect(getGameStatus).toHaveBeenCalledWith()
    expect(listLevels).not.toHaveBeenCalled()
    expect(store.runtimeLevels).toEqual(runtimeLevels)
    expect(store.runtimeLoading).toBe(false)
  })

  it('loads cluster options without exposing request-target selection state', async () => {
    const clusters: ClusterSummary[] = [
      {
        clusterName: 'Cluster_1',
      },
      {
        clusterName: 'Cluster_2',
      },
    ]
    listClusters.mockResolvedValue(success(clusters))

    const store = useClusterStore()

    await expect(store.refreshClusters()).resolves.toEqual(clusters)

    expect(store.clusters).toEqual(clusters)
    expect('selectedCluster' in store).toBe(false)
    expect('setSelectedCluster' in store).toBe(false)
  })

  it('normalizes clusters from paged backend responses', async () => {
    const clusters: ClusterSummary[] = [
      {
        clusterName: 'Cluster_1',
      },
    ]
    listClusters.mockResolvedValue(success({ data: clusters, total: 1 }))

    const store = useClusterStore()

    await expect(store.refreshClusters()).resolves.toEqual(clusters)

    expect(store.clusters).toEqual(clusters)
  })

  it('uses theme defaults when browser storage is unavailable', () => {
    const getItem = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('storage unavailable')
    })

    try {
      const store = useThemeStore()

      expect(store.mode).toBe('light')
      expect(store.primaryColor).toBe('#409eff')
    } finally {
      getItem.mockRestore()
    }
  })
})
