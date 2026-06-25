import { ref } from 'vue'
import { defineStore } from 'pinia'

import { getGameStatus } from '@/features/game/game.api'
import { listLevels } from '@/features/levels/level.api'
import { isApiSuccess } from '@/shared/api/http'
import type { LevelSummary } from '@/shared/types/domain'

export const useLevelStore = defineStore('levels', () => {
  const levels = ref<LevelSummary[]>([])
  const loading = ref(false)
  const runtimeLevels = ref<LevelSummary[]>([])
  const runtimeLoading = ref(false)

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

  async function refreshRuntimeLevels(cluster?: string): Promise<LevelSummary[]> {
    runtimeLoading.value = true

    try {
      const response = await getGameStatus(cluster)
      runtimeLevels.value = isApiSuccess(response) ? response.data : []
      return runtimeLevels.value
    } finally {
      runtimeLoading.value = false
    }
  }

  return {
    levels,
    loading,
    runtimeLevels,
    runtimeLoading,
    refreshLevels,
    refreshRuntimeLevels,
  }
})
