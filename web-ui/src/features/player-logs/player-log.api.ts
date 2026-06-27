import { apiGet, apiPost } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'

export interface PlayerLogRecord {
  ID?: number
  action?: string
  createdAt?: string
  created_at?: string
  id?: number
  ip?: string
  kuId?: string
  ku_id?: string
  name?: string
  role?: string
  steamId?: string
  steam_id?: string
}

export interface PlayerLogPage {
  data: PlayerLogRecord[]
  page: number
  size: number
  total: number
  totalPages: number
}

export interface PlayerLogQuery {
  action?: string
  kuId?: string
  name?: string
  page?: number
  role?: string
  size?: number
  steamId?: string
}

export function getPlayerLogs(query: PlayerLogQuery = {}): Promise<ApiEnvelope<PlayerLogPage>> {
  const params = new URLSearchParams()
  Object.entries({ page: 1, size: 10, ...query }).forEach(([key, value]) => {
    if (value !== undefined && value !== '') {
      params.set(key, String(value))
    }
  })
  return apiGet<ApiEnvelope<PlayerLogPage>>(`/api/player/log?${params.toString()}`)
}

export function deletePlayerLogs(ids: number[]): Promise<ApiEnvelope<unknown>> {
  return apiPost<ApiEnvelope<unknown>, { ids: number[] }>('/api/player/log/delete', { ids })
}
