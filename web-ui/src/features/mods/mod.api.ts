import { apiDelete, apiGet, apiPost, apiPut } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { ModSummary } from '@/shared/types/domain'

export interface ModQuery {
  keyword?: string
  modId?: string
  page?: number
  size?: number
  [key: string]: unknown
}

export type ModPayload = Partial<ModSummary> & Record<string, unknown>

export function listMods(params?: ModQuery): Promise<ApiEnvelope<ModSummary[]>> {
  return apiGet('/api/mod', { params })
}

export function searchMods(params?: ModQuery): Promise<ApiEnvelope<ModSummary[]>> {
  return apiGet('/api/mod/search', { params })
}

export function getMod(id: string | number): Promise<ApiEnvelope<ModSummary>> {
  return apiGet(`/api/mod/${encodeURIComponent(String(id))}`)
}

export function updateMod(
  id: string | number,
  payload: ModPayload,
): Promise<ApiEnvelope<ModSummary>> {
  return apiPut(`/api/mod/${encodeURIComponent(String(id))}`, payload)
}

export function deleteMod(id: string | number): Promise<ApiEnvelope<null>> {
  return apiDelete(`/api/mod/${encodeURIComponent(String(id))}`)
}

export function saveModInfo(payload: ModPayload): Promise<ApiEnvelope<ModSummary>> {
  return apiPost('/api/mod/modinfo', payload)
}

export function updateModInfo(payload: ModPayload): Promise<ApiEnvelope<ModSummary>> {
  return apiPut('/api/mod/modinfo', payload)
}
