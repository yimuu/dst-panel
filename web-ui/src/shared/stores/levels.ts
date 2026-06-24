import { ref } from 'vue'
import { defineStore } from 'pinia'

import { listLevels } from '@/features/levels/level.api'
import { isApiSuccess } from '@/shared/api/http'
import type { LevelSummary } from '@/shared/types/domain'

export const useLevelStore = defineStore('levels', () => {
  const levels = ref<LevelSummary[]>([])
  const loading = ref(false)

  async function refreshLevels(cluster?: string): Promise<LevelSummary[]> {
    loading.value = true

    try {
      const response = await listLevels(cluster)
      levels.value = isApiSuccess(response) ? response.data : []
      return levels.value
    } finally {
      loading.value = false
    }
  }

  return {
    levels,
    loading,
    refreshLevels,
  }
})
