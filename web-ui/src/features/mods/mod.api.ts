import { apiDelete, apiGet, apiPost, apiPut } from '@/shared/api/http'
import type { ApiEnvelope, PageResult } from '@/shared/api/types'
import type { ModSummary } from '@/shared/types/domain'

export interface ModQuery {
  text?: string
  page?: number
  size?: number
  lang?: string
}

export type ModPayload = Partial<ModSummary> & Record<string, unknown>

export function listMods(params?: ModQuery): Promise<ApiEnvelope<ModSummary[]>> {
  return apiGet('/api/mod', { params })
}

export function searchMods(params?: ModQuery): Promise<ApiEnvelope<PageResult<ModSummary>>> {
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

export function deleteMod(id: string | number): Promise<ApiEnvelope<string>> {
  return apiDelete(`/api/mod/${encodeURIComponent(String(id))}`)
}

export function saveModInfo(payload: ModPayload): Promise<ApiEnvelope<ModSummary>> {
  return apiPost('/api/mod/modinfo', payload)
}

export function updateModInfo(payload: ModPayload): Promise<ApiEnvelope<ModSummary>> {
  return apiPut('/api/mod/modinfo', payload)
}

export function uploadUgcMod(payload: FormData): Promise<ApiEnvelope<null>> {
  return apiPost('/api/file/ugc/upload', payload)
}

export function deleteSetupWorkshop(): Promise<ApiEnvelope<null>> {
  return apiDelete('/api/mod/setup/workshop')
}

export function readUgcAcf(levelName = 'Master'): Promise<ApiEnvelope<ModSummary[]>> {
  return apiGet('/api/mod/ugc/acf', {
    params: { levelName },
  })
}

export function deleteUgcMod(workshopId: string, levelName = 'Master'): Promise<ApiEnvelope<null>> {
  return apiDelete('/api/mod/ugc', {
    params: {
      levelName,
      workshopId,
    },
  })
}
