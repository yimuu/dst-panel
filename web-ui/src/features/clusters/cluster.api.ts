import { apiDelete, apiGet, apiPost, apiPut } from '@/shared/api/http'
import type { ApiEnvelope, PageResult } from '@/shared/api/types'
import type { ClusterSummary } from '@/shared/types/domain'

export interface ClusterListParams {
  page?: number
  size?: number
}

export type ClusterPayload = Partial<ClusterSummary> & Record<string, unknown>

export function listClusters(
  params?: ClusterListParams,
): Promise<ApiEnvelope<PageResult<ClusterSummary> | ClusterSummary[]>> {
  return apiGet('/api/cluster', { params })
}

export function createCluster(payload: ClusterPayload): Promise<ApiEnvelope<null>> {
  return apiPost('/api/cluster', payload)
}

export function updateCluster(payload: ClusterPayload): Promise<ApiEnvelope<null>> {
  return apiPut('/api/cluster', payload)
}

export function deleteCluster(id: number | string): Promise<ApiEnvelope<null>> {
  return apiDelete('/api/cluster', { params: { id } })
}
