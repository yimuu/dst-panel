import { apiGet, apiPost } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { GameConfig } from '@/shared/types/domain'

export interface SystemInfo {
  host: HostInfo
  cpu: CpuInfo
  mem: MemInfo
  disk: DiskInfo
  panelMemUsage: number
  panelCpuUsage: number
}

export interface HostInfo {
  os: string
  hostname: string
  platform: string
  kernelArch: string
}

export interface CpuInfo {
  cores: number
  cpuPercent: number[]
  cpuUsedPercent: number
  cpuUsed: number
}

export interface MemInfo {
  total: number
  available: number
  used: number
  usedPercent: number
}

export interface DiskInfo {
  devices: DeviceInfo[]
}

export interface DeviceInfo {
  device: string
  mountpoint: string
  fstype: string
  opts: string
  total: number
  usage: number
  inodesUsage: number
}

export interface DstProcessInfo {
  cpuUage: string
  memUage: string
  VSZ: string
  RSS: string
}

export interface LevelStatusInfo {
  Ps: DstProcessInfo
  status: boolean
  levelName: string
  is_master: boolean
  uuid: string
  leveldataoverride: string
  modoverrides: string
  server_ini: unknown
}

export interface OnlinePlayer {
  key: string
  day: string
  name: string
  kuId: string
  role: string
}

export interface GameCommandPayload {
  levelName: string
  command: string
}

export function getLevelStatus(): Promise<ApiEnvelope<LevelStatusInfo[]>> {
  return apiGet<ApiEnvelope<LevelStatusInfo[]>>('/api/game/8level/status')
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

export function getOnlinePlayers(levelName: string): Promise<ApiEnvelope<OnlinePlayer[]>> {
  return apiGet<ApiEnvelope<OnlinePlayer[]>>(
    `/api/game/8level/players?levelName=${encodeURIComponent(levelName)}`,
  )
}

export function getAllOnlinePlayers(): Promise<ApiEnvelope<OnlinePlayer[]>> {
  return apiGet<ApiEnvelope<OnlinePlayer[]>>('/api/game/8level/players/all')
}

export function getLevelServerLog(
  levelName: string,
  lines = 80,
): Promise<ApiEnvelope<string[]>> {
  const params = new URLSearchParams({ levelName, lines: String(lines) })
  return apiGet<ApiEnvelope<string[]>>(`/api/game/level/server/log?${params.toString()}`)
}

export function getLevelLogDownloadUrl(levelName: string, fileName = 'server_log.txt'): string {
  return `/api/game/level/server/download?${new URLSearchParams({
    levelName,
    fileName,
  }).toString()}`
}

export function rollbackGame(dayNums: number): Promise<ApiEnvelope<unknown>> {
  return apiGet<ApiEnvelope<unknown>>(
    `/api/game/rollback?dayNums=${encodeURIComponent(String(dayNums))}`,
  )
}

export function regenerateWorld(): Promise<ApiEnvelope<unknown>> {
  return apiGet<ApiEnvelope<unknown>>('/api/game/regenerateworld')
}

export function getSystemInfo(): Promise<ApiEnvelope<SystemInfo>> {
  return apiGet<ApiEnvelope<SystemInfo>>('/api/game/system/info')
}

export function getGameConfig(): Promise<ApiEnvelope<GameConfig>> {
  return apiGet<ApiEnvelope<GameConfig>>('/api/game/config')
}

export function saveGameConfig(config: Partial<GameConfig>): Promise<ApiEnvelope<unknown>> {
  return apiPost<ApiEnvelope<unknown>, Partial<GameConfig>>('/api/game/config', config)
}

export function updateGame(): Promise<ApiEnvelope<unknown>> {
  return apiGet<ApiEnvelope<unknown>>('/api/game/update')
}

export function createGameBackup(): Promise<ApiEnvelope<unknown>> {
  return apiPost<ApiEnvelope<unknown>>('/api/game/backup')
}
