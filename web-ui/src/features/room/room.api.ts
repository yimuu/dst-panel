import { apiDelete, apiGet, apiPost } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'

import type { PlayerListKind } from './player-lists'

export interface ClusterIni {
  game_mode: string
  max_players: number
  pvp: boolean
  pause_when_nobody: boolean
  vote_enabled: boolean
  vote_kick_enabled: boolean
  lan_only_cluster: boolean
  cluster_intention: string
  cluster_description: string
  cluster_password: string
  cluster_name: string
  offline_cluster: boolean
  cluster_language: string
  whitelist_slots: number
  tick_rate: number
  console_enabled: boolean
  max_snapshots: number
  shard_enabled: boolean
  bind_ip: string
  master_ip: string
  master_port: number
  cluster_key: string
  steam_group_id: string
  steam_group_only: boolean
  steam_group_admins: boolean
}

export interface ClusterIniEnvelope {
  cluster: ClusterIni
  token: string
}

export type MutablePlayerListKind = Exclude<PlayerListKind, 'whitelist'>

const playerListRoutes: Record<PlayerListKind, string> = {
  adminlist: '/api/game/8level/adminilist',
  whitelist: '/api/game/8level/whitelist',
  blacklist: '/api/game/8level/blacklist',
}

const mutablePlayerListRoutes: Record<MutablePlayerListKind, string> = {
  adminlist: '/api/game/player/adminlist',
  blacklist: '/api/game/player/blacklist',
}

const playerListPayloadKeys: Record<PlayerListKind, 'adminList' | 'whitelist' | 'blacklist'> = {
  adminlist: 'adminList',
  whitelist: 'whitelist',
  blacklist: 'blacklist',
}

export function getClusterIni(): Promise<ApiEnvelope<ClusterIniEnvelope>> {
  return apiGet<ApiEnvelope<ClusterIniEnvelope>>('/api/game/8level/clusterIni')
}

export function saveClusterIni(
  payload: ClusterIniEnvelope,
): Promise<ApiEnvelope<ClusterIniEnvelope>> {
  return apiPost<ApiEnvelope<ClusterIniEnvelope>, ClusterIniEnvelope>(
    '/api/game/8level/clusterIni',
    payload,
  )
}

export function getPlayerList(kind: PlayerListKind): Promise<ApiEnvelope<string[]>> {
  return apiGet<ApiEnvelope<string[]>>(playerListRoutes[kind])
}

export function savePlayerList(
  kind: PlayerListKind,
  values: string[],
): Promise<ApiEnvelope<unknown>> {
  return apiPost<ApiEnvelope<unknown>, PlayerListPayload>(playerListRoutes[kind], {
    [playerListPayloadKeys[kind]]: values,
  })
}

export function addPlayerListEntries(
  kind: MutablePlayerListKind,
  values: string[],
): Promise<ApiEnvelope<unknown>> {
  return apiPost<ApiEnvelope<unknown>, PlayerListPayload>(mutablePlayerListRoutes[kind], {
    [playerListPayloadKeys[kind]]: values,
  })
}

export function removePlayerListEntries(
  kind: MutablePlayerListKind,
  values: string[],
): Promise<ApiEnvelope<unknown>> {
  return apiDelete<ApiEnvelope<unknown>>(mutablePlayerListRoutes[kind], {
    data: {
      [playerListPayloadKeys[kind]]: values,
    },
  })
}

type PlayerListPayload = Partial<Record<'adminList' | 'whitelist' | 'blacklist', string[]>>
