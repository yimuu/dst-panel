import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import * as clusterApi from '@/features/clusters/cluster.api'
import type { ApiEnvelope } from '@/shared/api/types'
import { useClusterStore } from '@/shared/stores/cluster'
import { useLevelStore } from '@/shared/stores/levels'
import { useThemeStore } from '@/shared/stores/theme'
import type { ClusterSummary } from '@/shared/types/domain'

vi.mock('@/features/clusters/cluster.api', () => ({
  listClusters: vi.fn(),
}))

const listClusters = vi.mocked(clusterApi.listClusters)

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
  })

  it('keeps the selected cluster as a string for API header consumers', async () => {
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
    expect(store.selectedCluster).toBe('')

    store.setSelectedCluster('Cluster_2')

    expect(store.selectedCluster).toBe('Cluster_2')
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
