import { apiGet, apiPost } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { ClusterIniEnvelope } from '@/shared/types/domain'

import {
  buildPlayerListPayload,
  playerListContracts,
  type PlayerListKind,
  type PlayerListPayload,
} from './player-lists'

export type { PlayerListKind } from './player-lists'

const clusterIniPath = '/api/game/8level/clusterIni'

export function getClusterIni(): Promise<ApiEnvelope<ClusterIniEnvelope>> {
  return apiGet(clusterIniPath)
}

export function saveClusterIni(
  payload: ClusterIniEnvelope,
): Promise<ApiEnvelope<ClusterIniEnvelope>> {
  return apiPost(clusterIniPath, payload)
}

export function getPlayerList(kind: PlayerListKind): Promise<ApiEnvelope<string[]>> {
  return apiGet(playerListContracts[kind].path)
}

export function savePlayerList(
  kind: PlayerListKind,
  values: string[],
): Promise<ApiEnvelope<null>> {
  const payload = buildPlayerListPayload(kind, values)

  return apiPost<null, PlayerListPayload>(playerListContracts[kind].path, payload)
}
