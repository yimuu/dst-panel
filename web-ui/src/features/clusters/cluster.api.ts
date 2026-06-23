import { apiDelete, apiGet, apiPost, apiPut } from '@/shared/api/http'
import type { ApiEnvelope, PageResult } from '@/shared/api/types'
import type { ClusterSummary } from '@/shared/types/domain'

export interface ClusterListParams {
  page?: number
  size?: number
}

export interface CreateClusterRequest {
  clusterName: string
  description: string
  steamcmd: string
  force_install_dir: string
  backup: string
  mod_download_path: string
  uuid: string
  beta: number | boolean
  bin: number
  ugc_directory: string
  persistent_storage_root: string
  conf_dir: string
}

export type UpdateClusterRequest = (
  | {
      ID: number
      id?: never
    }
  | {
      ID?: never
      id: number
    }
) & {
  description?: string
}

export function listClusters(
  params?: ClusterListParams,
): Promise<ApiEnvelope<PageResult<ClusterSummary> | ClusterSummary[]>> {
  return apiGet('/api/cluster', { params })
}

export function createCluster(payload: CreateClusterRequest): Promise<ApiEnvelope<null>> {
  return apiPost('/api/cluster', payload)
}

export function updateCluster(payload: UpdateClusterRequest): Promise<ApiEnvelope<null>> {
  return apiPut('/api/cluster', payload)
}

export function deleteCluster(id: number | string): Promise<ApiEnvelope<null>> {
  return apiDelete('/api/cluster', { params: { id } })
}
