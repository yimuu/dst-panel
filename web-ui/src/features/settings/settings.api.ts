import { apiDelete, apiGet, apiPost } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { TaskSummary } from '@/shared/types/domain'

import type { PanelSettingsForm } from './settings-form'

export type SettingsPayload = PanelSettingsForm

export type DstConfigResponse = Partial<PanelSettingsForm> & Record<string, unknown>

export interface LobbyDetailRequest {
  region: string
  rowId: string
}

export interface LobbyPlayer {
  name?: string
  prefab?: string
  colour?: string
  color?: string
  [key: string]: unknown
}

export interface LobbyDayData {
  day?: number
  dayselapsedinseason?: number
  daysleftinseason?: number
  [key: string]: unknown
}

export interface LobbyServerDetail {
  __addr?: string
  __rowId?: string
  name?: string
  desc?: string
  host?: string
  mode?: string
  season?: string
  connected?: number
  maxconnections?: number
  password?: boolean
  dedicated?: boolean
  mods?: boolean
  pvp?: boolean
  playerList?: LobbyPlayer[] | null
  dayData?: LobbyDayData | null
  [key: string]: unknown
}

export type TaskCategory =
  | 'backup'
  | 'update'
  | 'start'
  | 'stop'
  | 'restart'
  | 'regenerate'
  | 'startGame'
  | 'stopGame'
  | 'none'

type OptionalBackendId =
  | {
      ID?: number
      id?: never
    }
  | {
      ID?: never
      id?: number
    }

export type SaveTaskRequest = OptionalBackendId & {
  clusterName?: string
  levelName?: string
  uuid?: string
  cron: string
  category: TaskCategory
  comment?: string
  announcement?: string
  sleep?: number
  times?: number
  script?: number
}

export type AutoCheckType = 'LEVEL_MOD' | 'LEVEL_DOWN' | 'UPDATE_GAME' | string

export interface AutoCheckSummary {
  ID?: number
  id?: number
  CreatedAt?: string | null
  UpdatedAt?: string | null
  DeletedAt?: string | null
  name?: string
  clusterName?: string
  levelName?: string
  uuid?: string
  enable?: number
  announcement?: string
  times?: number
  sleep?: number
  interval?: number
  checkType?: AutoCheckType
}

export type AutoCheckPayload = OptionalBackendId & {
  name?: string
  clusterName?: string
  levelName?: string
  uuid?: string
  enable?: number
  announcement?: string
  times?: number
  sleep?: number
  interval?: number
  checkType: AutoCheckType
}

export function getDstConfig(): Promise<ApiEnvelope<DstConfigResponse>> {
  return apiGet('/api/dst/config')
}

export function saveDstConfig(
  payload: SettingsPayload,
): Promise<ApiEnvelope<DstConfigResponse | null>> {
  return apiPost('/api/dst/config', payload)
}

export function getLobbyServerDetail(
  params: LobbyDetailRequest,
): Promise<ApiEnvelope<LobbyServerDetail>> {
  return apiGet('/api/dst/lobby/server/detail', {
    params,
  })
}

export function listTasks(): Promise<ApiEnvelope<TaskSummary[]>> {
  return apiGet('/api/task')
}

export function saveTask(payload: SaveTaskRequest): Promise<ApiEnvelope<null>> {
  return apiPost('/api/task', payload)
}

export function deleteTask(jobId: number | string): Promise<ApiEnvelope<null>> {
  return apiDelete('/api/task', { params: { jobId } })
}

export function getAutoCheck(checkType?: AutoCheckType): Promise<ApiEnvelope<AutoCheckSummary[]>> {
  return apiGet('/api/auto/check2', { params: { checkType } })
}

export function saveAutoCheck(payload: AutoCheckPayload): Promise<ApiEnvelope<AutoCheckSummary>> {
  return apiPost('/api/auto/check2', payload)
}
