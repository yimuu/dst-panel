import { ref } from 'vue'
import { defineStore } from 'pinia'

import { listClusters } from '@/features/clusters/cluster.api'
import { isApiSuccess } from '@/shared/api/http'
import type { ClusterSummary } from '@/shared/types/domain'

export const useClusterStore = defineStore('cluster', () => {
  const selectedCluster = ref('')
  const clusters = ref<ClusterSummary[]>([])
  const loading = ref(false)

  async function refreshClusters(): Promise<ClusterSummary[]> {
    loading.value = true

    try {
      const response = await listClusters()
      const data = isApiSuccess(response) ? response.data : []
      clusters.value = Array.isArray(data) ? data : data.data || []

      return clusters.value
    } finally {
      loading.value = false
    }
  }

  function setSelectedCluster(cluster: string): void {
    selectedCluster.value = cluster
  }

  return {
    selectedCluster,
    clusters,
    loading,
    refreshClusters,
    setSelectedCluster,
  }
})
