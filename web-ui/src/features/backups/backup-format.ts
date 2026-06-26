export type BackupAction = 'create' | 'restore' | 'delete'

export function formatBackupSize(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`
  }

  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`
  }

  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

export function getBackupActionLabel(action: BackupAction): string {
  return {
    create: '创建备份',
    restore: '恢复',
    delete: '删除',
  }[action]
}
