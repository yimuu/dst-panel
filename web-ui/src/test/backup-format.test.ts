import { describe, expect, it } from 'vitest'

import { formatBackupSize, formatBackupTime } from '@/features/backups/backup-format'

describe('backup format', () => {
  it('formats byte sizes for backup tables', () => {
    expect(formatBackupSize(0)).toBe('0 B')
    expect(formatBackupSize(1024)).toBe('1.00 KB')
    expect(formatBackupSize(1024 * 1024)).toBe('1.00 MB')
  })

  it('formats unix timestamps for backup tables', () => {
    expect(formatBackupTime(1712828023)).toContain('2024')
    expect(formatBackupTime(0)).toBe('-')
  })
})
