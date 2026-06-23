import { apiGet, apiPost, withCluster } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { LevelSummary } from '@/shared/types/domain'

export interface GameCommandRequest {
  command: string
  levelName?: string
  [key: string]: unknown
}

export function getGameStatus(cluster?: string): Promise<ApiEnvelope<LevelSummary[]>> {
  return apiGet('/api/game/8level/status', withCluster(cluster))
}

export function startLevel(levelName: string, cluster?: string): Promise<ApiEnvelope<null>> {
  return apiGet('/api/game/8level/start', {
    ...withCluster(cluster),
    params: { levelName },
  })
}

export function stopLevel(levelName: string, cluster?: string): Promise<ApiEnvelope<null>> {
  return apiGet('/api/game/8level/stop', {
    ...withCluster(cluster),
    params: { levelName },
  })
}

export function startGame(levelName: string, cluster?: string): Promise<ApiEnvelope<null>> {
  return startLevel(levelName, cluster)
}

export function stopGame(levelName: string, cluster?: string): Promise<ApiEnvelope<null>> {
  return stopLevel(levelName, cluster)
}

export function sendGameCommand(
  payload: GameCommandRequest,
  cluster?: string,
): Promise<ApiEnvelope<null>> {
  return apiPost('/api/game/8level/command', payload, withCluster(cluster))
}

export function getSystemInfo(cluster?: string): Promise<ApiEnvelope<Record<string, unknown>>> {
  return apiGet('/api/game/system/info', withCluster(cluster))
}
