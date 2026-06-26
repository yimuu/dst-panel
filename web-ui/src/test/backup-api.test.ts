import { afterEach, describe, expect, it, vi } from 'vitest'

import { restoreBackup } from '@/features/backups/backup.api'
import { http } from '@/shared/api/http'

const successResponse = { data: { code: 0, data: null } }

describe('backup API wrappers', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('passes backupName without unsupported cluster headers', async () => {
    const get = vi.spyOn(http, 'get').mockResolvedValue(successResponse)

    await restoreBackup('x.zip')

    expect(get).toHaveBeenCalledWith('/api/game/backup/restore', {
      params: {
        backupName: 'x.zip',
      },
    })
  })
})
