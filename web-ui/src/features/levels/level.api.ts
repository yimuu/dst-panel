import { apiDelete, apiGet, apiPost, apiPut, withCluster } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { LevelSummary } from '@/shared/types/domain'

export type LevelPayload = Partial<LevelSummary> & Record<string, unknown>

export function listLevels(cluster?: string): Promise<ApiEnvelope<LevelSummary[]>> {
  return apiGet('/api/cluster/level', withCluster(cluster))
}

export function createLevel(
  payload: LevelPayload,
  cluster?: string,
): Promise<ApiEnvelope<LevelSummary>> {
  return apiPost('/api/cluster/level', payload, withCluster(cluster))
}

export function saveLevels(levels: LevelSummary[], cluster?: string): Promise<ApiEnvelope<null>> {
  return apiPut('/api/cluster/level', { levels }, withCluster(cluster))
}

export function deleteLevel(levelName: string, cluster?: string): Promise<ApiEnvelope<null>> {
  return apiDelete('/api/cluster/level', {
    ...withCluster(cluster),
    params: { levelName },
  })
}
