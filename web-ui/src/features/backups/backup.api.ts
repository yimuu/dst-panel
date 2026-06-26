import { apiDelete, apiGet, apiPost, apiPut, http } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { BackupFile } from '@/shared/types/domain'

export interface CreateBackupRequest {
  backupName?: string
  [key: string]: unknown
}

export interface DeleteBackupsRequest {
  fileNames: string[]
  [key: string]: unknown
}

export interface RenameBackupRequest {
  fileName: string
  newName: string
  [key: string]: unknown
}

export function listBackups(): Promise<ApiEnvelope<BackupFile[]>> {
  return apiGet('/api/game/backup')
}

export function createBackup(payload?: CreateBackupRequest): Promise<ApiEnvelope<null>> {
  return apiPost('/api/game/backup', payload)
}

export function deleteBackups(payload: DeleteBackupsRequest): Promise<ApiEnvelope<null>> {
  return apiDelete('/api/game/backup', {
    data: payload,
  })
}

export function restoreBackup(backupName: string): Promise<ApiEnvelope<null>> {
  return apiGet('/api/game/backup/restore', {
    params: { backupName },
  })
}

export function renameBackup(payload: RenameBackupRequest): Promise<ApiEnvelope<null>> {
  return apiPut('/api/game/backup', payload)
}

export function uploadBackup(file: File): Promise<ApiEnvelope<null>> {
  const formData = new FormData()
  formData.append('file', file)

  return apiPost('/api/game/backup/upload', formData)
}

export async function downloadBackup(fileName: string): Promise<Blob> {
  const response = await http.get<Blob>('/api/game/backup/download', {
    params: { fileName },
    responseType: 'blob',
  })
  return response.data
}
