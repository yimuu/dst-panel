import { describe, expect, it } from 'vitest'

import { formatBackupSize, getBackupActionLabel } from '@/features/backups/backup-format'

describe('backup formatting', () => {
  it('formats byte sizes for table display', () => {
    expect(formatBackupSize(0)).toBe('0 B')
    expect(formatBackupSize(1024)).toBe('1.0 KB')
    expect(formatBackupSize(1048576)).toBe('1.0 MB')
  })

  it('uses Chinese action labels', () => {
    expect(getBackupActionLabel('create')).toBe('创建备份')
    expect(getBackupActionLabel('restore')).toBe('恢复')
    expect(getBackupActionLabel('delete')).toBe('删除')
  })
})
