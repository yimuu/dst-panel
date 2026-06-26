import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  downloadBackup,
  renameBackup,
  restoreBackup,
  uploadBackup,
} from '@/features/backups/backup.api'
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

  it('renames backups with the backend PUT body shape', async () => {
    const put = vi.spyOn(http, 'put').mockResolvedValue(successResponse)
    const payload = { fileName: 'old.zip', newName: 'new.zip' }

    await renameBackup(payload)

    expect(put).toHaveBeenCalledWith('/api/game/backup', payload, undefined)
  })

  it('downloads backups as blobs', async () => {
    const blob = new Blob(['backup'])
    const get = vi.spyOn(http, 'get').mockResolvedValue({ data: blob })

    await downloadBackup('backup.zip')

    expect(get).toHaveBeenCalledWith('/api/game/backup/download', {
      params: {
        fileName: 'backup.zip',
      },
      responseType: 'blob',
    })
  })

  it('uploads backups with multipart form data', async () => {
    const post = vi.spyOn(http, 'post').mockResolvedValue(successResponse)
    const file = new File(['backup'], 'backup.zip')

    await uploadBackup(file)

    expect(post).toHaveBeenCalledWith('/api/game/backup/upload', expect.any(FormData), undefined)
    expect((post.mock.calls[0]?.[1] as FormData).get('file')).toBe(file)
  })
})
