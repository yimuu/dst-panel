import { describe, expect, it } from 'vitest'

import { createDefaultDstConfig, validateDstConfig } from '@/features/settings/settings-form'

describe('settings form', () => {
  it('rejects missing required fields', () => {
    expect(
      validateDstConfig({
        steamcmd: '',
        force_install_dir: '',
        backup: '',
        mod_download_path: '',
        cluster: '',
        persistent_storage_root: '',
        conf_dir: '',
        ugc_directory: '',
        donot_starve_server_directory: '',
        bin: '32',
        beta: 0,
      }),
    ).toContain('steamcmd')
  })

  it('creates a complete default config for forms', () => {
    expect(createDefaultDstConfig()).toMatchObject({
      bin: '32',
      beta: 0,
      cluster: 'Cluster1',
    })
  })
})
