import { apiGet, apiPost } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { LevelSummary } from '@/shared/types/domain'

export interface GameCommandRequest {
  command: string
  levelName?: string
  [key: string]: unknown
}

export function getGameStatus(): Promise<ApiEnvelope<LevelSummary[]>> {
  return apiGet('/api/game/8level/status')
}

export function startLevel(levelName: string): Promise<ApiEnvelope<null>> {
  return apiGet('/api/game/8level/start', {
    params: { levelName },
  })
}

export function stopLevel(levelName: string): Promise<ApiEnvelope<null>> {
  return apiGet('/api/game/8level/stop', {
    params: { levelName },
  })
}

export function startGame(levelName: string): Promise<ApiEnvelope<null>> {
  return startLevel(levelName)
}

export function stopGame(levelName: string): Promise<ApiEnvelope<null>> {
  return stopLevel(levelName)
}

export function sendGameCommand(payload: GameCommandRequest): Promise<ApiEnvelope<null>> {
  return apiPost('/api/game/8level/command', payload)
}

export function applyPreinstallTemplate(name: string): Promise<ApiEnvelope<null>> {
  return apiGet('/api/game/preinstall', {
    params: { name },
  })
}

export function getSystemInfo(): Promise<ApiEnvelope<Record<string, unknown>>> {
  return apiGet('/api/game/system/info')
}

export function buildGameLogStreamPath(levelName: string): string {
  const params = new URLSearchParams({ levelName })
  return `/api/game/log/stream?${params.toString()}`
}
