import { formatWorkshopId, type ModConfigEntry } from './mod-model'

import { apiDelete, apiGet, apiPost, apiPut } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'

export interface ModInfoRecord {
  ID?: number
  auth?: string
  consumer_appid?: number
  consumer_id?: number
  creator_appid?: number
  description?: string
  enabled?: boolean
  file_url?: string
  img?: string
  last_time?: number
  mod_config?: ModConfigEntry[] | string | Record<string, unknown>
  modid: string
  name?: string
  update?: boolean
  v?: string
  [key: string]: unknown
}

export interface ModSearchItem {
  author?: string
  created?: number
  id?: string
  img?: string
  modid?: string
  name?: string
  score?: number
  sub?: number | string
  subscription?: string
  time?: number | string
  vote?: {
    num?: number
    star?: number
  }
  [key: string]: unknown
}

export interface ModSearchResponse {
  data: ModSearchItem[]
  page?: number
  size?: number
  total?: number
  totalPage?: number
}

export interface ManualModInfoPayload {
  modinfo: string
  workshopId: string
}

export interface UgcModInfo {
  img?: string
  name?: string
  timelast?: number
  timeupdated?: number
  workshopId: string
  [key: string]: unknown
}

export function getMods(): Promise<ApiEnvelope<ModInfoRecord[]>> {
  return apiGet<ApiEnvelope<ModInfoRecord[]>>('/api/mod')
}

export function saveModInfo(record: ModInfoRecord): Promise<ApiEnvelope<ModInfoRecord>> {
  return apiPost<ApiEnvelope<ModInfoRecord>, ModInfoRecord>(
    '/api/mod/modinfo',
    toRawModInfoPayload(record),
  )
}

export function updateAllModInfo(lang = 'zh'): Promise<ApiEnvelope<unknown>> {
  return apiPut<ApiEnvelope<unknown>>(`/api/mod/modinfo?${new URLSearchParams({ lang })}`)
}

export function subscribeMod(modId: string, lang = 'zh'): Promise<ApiEnvelope<ModInfoRecord>> {
  const normalizedModId = formatWorkshopId(modId)
  return apiGet<ApiEnvelope<ModInfoRecord>>(
    `/api/mod/${encodeURIComponent(normalizedModId)}?${new URLSearchParams({ lang })}`,
  )
}

export function updateMod(modId: string, lang = 'zh'): Promise<ApiEnvelope<ModInfoRecord>> {
  const normalizedModId = formatWorkshopId(modId)
  return apiPut<ApiEnvelope<ModInfoRecord>>(
    `/api/mod/${encodeURIComponent(normalizedModId)}?${new URLSearchParams({ lang })}`,
  )
}

export function deleteMod(modId: string): Promise<ApiEnvelope<unknown>> {
  return apiDelete<ApiEnvelope<unknown>>(`/api/mod/${encodeURIComponent(formatWorkshopId(modId))}`)
}

export function searchMods(
  text: string,
  page = 1,
  size = 10,
  lang = 'zh',
): Promise<ApiEnvelope<ModSearchResponse>> {
  const query = new URLSearchParams({
    text,
    page: String(page),
    size: String(size),
    lang,
  })
  return apiGet<ApiEnvelope<ModSearchResponse>>(`/api/mod/search?${query.toString()}`)
}

export function uploadModInfoFile(
  payload: ManualModInfoPayload,
  lang = 'zh',
): Promise<ApiEnvelope<unknown>> {
  return apiPost<ApiEnvelope<unknown>, ManualModInfoPayload>(
    `/api/mod/modinfo/file?${new URLSearchParams({ lang })}`,
    payload,
  )
}

export function getUgcMods(levelName: string): Promise<ApiEnvelope<UgcModInfo[]>> {
  return apiGet<ApiEnvelope<UgcModInfo[]>>(`/api/mod/ugc/acf?${new URLSearchParams({ levelName })}`)
}

export function deleteUgcMod(levelName: string, workshopId: string): Promise<ApiEnvelope<unknown>> {
  return apiDelete<ApiEnvelope<unknown>>(
    `/api/mod/ugc?${new URLSearchParams({
      levelName,
      workshopId: formatWorkshopId(workshopId),
    })}`,
  )
}

function toRawModInfoPayload(record: ModInfoRecord): ModInfoRecord {
  return {
    ...record,
    mod_config:
      typeof record.mod_config === 'string'
        ? record.mod_config
        : JSON.stringify(record.mod_config ?? {}),
  }
}
