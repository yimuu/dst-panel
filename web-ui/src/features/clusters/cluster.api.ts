import { apiDelete, apiGet, apiPost, apiPut } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'

export interface ClusterListQuery {
  clusterName?: string
  page?: number
  size?: number
}

export interface ClusterPage<T> {
  data: T[]
  total: number
  totalPages: number
  page: number
  size: number
}

export interface ClusterSummary {
  ID: number
  CreatedAt?: string | null
  UpdatedAt?: string | null
  clusterName: string
  description: string
  steamcmd: string
  force_install_dir: string
  backup: string
  mod_download_path: string
  uuid: string
  beta: number
  master: boolean
  caves: boolean
  rowId: string
  connected: number
  maxConnections: number
  mode: string
  mods: number
  season: string
  password: string
  region: string
}

export interface ClusterPayload {
  clusterName: string
  description: string
  steamcmd: string
  force_install_dir: string
  backup: string
  mod_download_path: string
  uuid: string
  beta: number
  bin: number
  ugc_directory: string
  persistent_storage_root: string
  conf_dir: string
}

export interface UpdateClusterPayload {
  ID: number
  description: string
}

export function getClusters(
  query: ClusterListQuery = {},
): Promise<ApiEnvelope<ClusterPage<ClusterSummary>>> {
  return apiGet<ApiEnvelope<ClusterPage<ClusterSummary>>>(
    withQuery('/api/cluster', {
      clusterName: query.clusterName,
      page: query.page,
      size: query.size,
    }),
  )
}

export function createCluster(payload: ClusterPayload): Promise<ApiEnvelope<unknown>> {
  return apiPost<ApiEnvelope<unknown>, ClusterPayload>('/api/cluster', payload)
}

export function updateCluster(payload: UpdateClusterPayload): Promise<ApiEnvelope<unknown>> {
  return apiPut<ApiEnvelope<unknown>, UpdateClusterPayload>('/api/cluster', payload)
}

export function deleteCluster(id: number): Promise<ApiEnvelope<unknown>> {
  return apiDelete<ApiEnvelope<unknown>>(withQuery('/api/cluster', { id }))
}

function withQuery(path: string, params: Record<string, string | number | undefined>): string {
  const search = new URLSearchParams()

  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined) {
      search.set(key, String(value))
    }
  }

  const query = search.toString()
  return query ? `${path}?${query}` : path
}
