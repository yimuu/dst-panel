import { afterEach, describe, expect, it, vi } from 'vitest'

import { restoreBackup } from '@/features/backups/backup.api'
import { http } from '@/shared/api/http'

const successResponse = { data: { code: 0, data: null } }

describe('backup API wrappers', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('passes backupName and cluster config to restore backup route', async () => {
    const get = vi.spyOn(http, 'get').mockResolvedValue(successResponse)

    await restoreBackup('x.zip', 'Cluster_1')

    expect(get).toHaveBeenCalledWith('/api/game/backup/restore', {
      headers: {
        Cluster: 'Cluster_1',
      },
      params: {
        backupName: 'x.zip',
      },
    })
  })
})
