import { afterEach, describe, expect, it, vi } from 'vitest'

import { saveDstConfig, type DstConfig } from '@/features/settings/settings.api'
import { http } from '@/shared/api/http'

const dstConfig: DstConfig = {
  steamcmd: '/opt/steamcmd',
  force_install_dir: '/srv/dst',
  donot_starve_server_directory: '',
  cluster: 'Cluster_1',
  backup: '/srv/backup',
  mod_download_path: '/srv/mods',
  bin: 64,
  beta: 0,
  ugc_directory: '',
  persistent_storage_root: '/srv/klei',
  conf_dir: 'DoNotStarveTogether',
}

describe('settings API', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('posts the backend DstConfig payload to the dst config endpoint', async () => {
    const post = vi.spyOn(http, 'post').mockResolvedValue({
      data: {
        code: 200,
        data: null,
      },
    })

    await saveDstConfig(dstConfig)

    expect(post).toHaveBeenCalledWith('/api/dst/config', dstConfig, undefined)
  })
})
