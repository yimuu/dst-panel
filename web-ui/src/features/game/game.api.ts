import { apiGet, apiPost } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { LevelSummary } from '@/shared/types/domain'

export interface SystemInfo {
  version?: string
  platform?: string
  memory?: {
    used?: number
    total?: number
    percent?: number
  }
  cpu?: {
    percent?: number
    cores?: number
  }
  disk?: {
    free?: number
    total?: number
    percent?: number
  }
  [key: string]: unknown
}

export interface GameCommandPayload {
  levelName?: string
  command: string
}

export function getLevelStatus(): Promise<ApiEnvelope<LevelSummary[]>> {
  return apiGet<ApiEnvelope<LevelSummary[]>>('/api/game/8level/status')
}

export function startLevel(levelName: string): Promise<ApiEnvelope<unknown>> {
  return apiGet<ApiEnvelope<unknown>>(
    `/api/game/8level/start?levelName=${encodeURIComponent(levelName)}`,
  )
}

export function stopLevel(levelName: string): Promise<ApiEnvelope<unknown>> {
  return apiGet<ApiEnvelope<unknown>>(
    `/api/game/8level/stop?levelName=${encodeURIComponent(levelName)}`,
  )
}

export function sendGameCommand(payload: GameCommandPayload): Promise<ApiEnvelope<unknown>> {
  return apiPost<ApiEnvelope<unknown>, GameCommandPayload>('/api/game/8level/command', payload)
}

export function getSystemInfo(): Promise<ApiEnvelope<SystemInfo>> {
  return apiGet<ApiEnvelope<SystemInfo>>('/api/game/system/info')
}

export function updateGame(): Promise<ApiEnvelope<unknown>> {
  return apiGet<ApiEnvelope<unknown>>('/api/game/update')
}

export function createGameBackup(): Promise<ApiEnvelope<unknown>> {
  return apiPost<ApiEnvelope<unknown>>('/api/game/backup')
}
