import { apiDelete, apiGet, apiPost, apiPut, http, withCluster } from '@/shared/api/http'
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

export function listBackups(cluster?: string): Promise<ApiEnvelope<BackupFile[]>> {
  return apiGet('/api/game/backup', withCluster(cluster))
}

export function createBackup(
  payload?: CreateBackupRequest,
  cluster?: string,
): Promise<ApiEnvelope<null>> {
  return apiPost('/api/game/backup', payload, withCluster(cluster))
}

export function deleteBackups(
  payload: DeleteBackupsRequest,
  cluster?: string,
): Promise<ApiEnvelope<null>> {
  return apiDelete('/api/game/backup', {
    ...withCluster(cluster),
    data: payload,
  })
}

export function renameBackup(
  payload: RenameBackupRequest,
  cluster?: string,
): Promise<ApiEnvelope<null>> {
  return apiPut('/api/game/backup', payload, withCluster(cluster))
}

export async function downloadBackup(fileName: string, cluster?: string): Promise<Blob> {
  const response = await http.get<Blob>('/api/game/backup/download', {
    ...withCluster(cluster),
    params: { fileName },
    responseType: 'blob',
  })
  return response.data
}
