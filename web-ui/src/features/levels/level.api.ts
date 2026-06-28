import { apiDelete, apiGet, apiPost, apiPut } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { ServerIniPayload } from '@/shared/types/domain'

export interface WorldLevel {
  levelName: string
  is_master: boolean
  uuid: string
  server_ini: ServerIniPayload
  leveldataoverride: string
  modoverrides: string
}

export function getLevels(): Promise<ApiEnvelope<WorldLevel[]>> {
  return apiGet<ApiEnvelope<WorldLevel[]>>('/api/cluster/level')
}

export function saveLevels(levels: WorldLevel[]): Promise<ApiEnvelope<unknown>> {
  return apiPut<ApiEnvelope<unknown>, { levels: WorldLevel[] }>('/api/cluster/level', { levels })
}

export function createLevel(level: WorldLevel): Promise<ApiEnvelope<WorldLevel>> {
  return apiPost<ApiEnvelope<WorldLevel>, WorldLevel>('/api/cluster/level', level)
}

export function deleteLevel(levelName: string): Promise<ApiEnvelope<unknown>> {
  return apiDelete<ApiEnvelope<unknown>>(
    `/api/cluster/level?${new URLSearchParams({ levelName }).toString()}`,
  )
}
