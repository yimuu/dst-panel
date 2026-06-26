import { apiDelete, apiGet, apiPost, apiPut } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { LevelSummary } from '@/shared/types/domain'

export type LevelPayload = Partial<LevelSummary> & Record<string, unknown>

export function listLevels(): Promise<ApiEnvelope<LevelSummary[]>> {
  return apiGet('/api/cluster/level')
}

export function createLevel(payload: LevelPayload): Promise<ApiEnvelope<LevelSummary>> {
  return apiPost('/api/cluster/level', payload)
}

export function saveLevels(levels: LevelSummary[]): Promise<ApiEnvelope<null>> {
  return apiPut('/api/cluster/level', { levels })
}

export function deleteLevel(levelName: string): Promise<ApiEnvelope<null>> {
  return apiDelete('/api/cluster/level', {
    params: { levelName },
  })
}
