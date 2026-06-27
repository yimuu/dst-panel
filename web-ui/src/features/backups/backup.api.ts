import { apiDelete, apiGet, apiPost, apiPut } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'

export interface BackupEntry {
  createTime?: string
  fileName: string
  fileSize: number
  time?: number
}

export interface BackupSnapshotSetting {
  ID?: number
  enable: number
  interval: number
  isCSave: number
  maxSnapshots: number
  name: string
}

export function getBackups(): Promise<ApiEnvelope<BackupEntry[]>> {
  return apiGet<ApiEnvelope<BackupEntry[]>>('/api/game/backup')
}

export function createBackup(backupName = ''): Promise<ApiEnvelope<unknown>> {
  return apiPost<ApiEnvelope<unknown>, { backupName: string }>('/api/game/backup', { backupName })
}

export function deleteBackups(fileNames: string[]): Promise<ApiEnvelope<unknown>> {
  return apiDelete<ApiEnvelope<unknown>>('/api/game/backup', { data: { fileNames } })
}

export function renameBackup(fileName: string, newName: string): Promise<ApiEnvelope<unknown>> {
  return apiPut<ApiEnvelope<unknown>, { fileName: string; newName: string }>('/api/game/backup', {
    fileName,
    newName,
  })
}

export function restoreBackup(backupName: string): Promise<ApiEnvelope<unknown>> {
  return apiGet<ApiEnvelope<unknown>>(
    `/api/game/backup/restore?${new URLSearchParams({ backupName })}`,
  )
}

export function getBackupDownloadUrl(fileName: string): string {
  return `/api/game/backup/download?${new URLSearchParams({ fileName })}`
}

export function uploadBackup(file: File): Promise<ApiEnvelope<unknown>> {
  const formData = new FormData()
  formData.append('file', file)
  return apiPost<ApiEnvelope<unknown>, FormData>('/api/game/backup/upload', formData)
}

export function getSnapshotSetting(): Promise<ApiEnvelope<BackupSnapshotSetting>> {
  return apiGet<ApiEnvelope<BackupSnapshotSetting>>('/api/game/backup/snapshot/setting')
}

export function saveSnapshotSetting(
  setting: BackupSnapshotSetting,
): Promise<ApiEnvelope<BackupSnapshotSetting>> {
  return apiPost<ApiEnvelope<BackupSnapshotSetting>, BackupSnapshotSetting>(
    '/api/game/backup/snapshot/setting',
    setting,
  )
}
