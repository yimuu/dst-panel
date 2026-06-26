import { describe, expect, it } from 'vitest'

import {
  createEmptyDstConfig,
  normalizeDstConfig,
  prepareDstConfigForSave,
  validateDstConfig,
} from '@/features/settings/settings-form'

describe('settings form', () => {
  it('creates an empty DstConfig with safe defaults', () => {
    expect(createEmptyDstConfig()).toEqual({
      steamcmd: '',
      force_install_dir: '',
      donot_starve_server_directory: '',
      cluster: '',
      backup: '',
      mod_download_path: '',
      bin: 32,
      beta: 0,
      ugc_directory: '',
      persistent_storage_root: '',
      conf_dir: '',
    })
  })

  it('normalizes loaded backend config conservatively', () => {
    expect(
      normalizeDstConfig({
        steamcmd: '  /opt/steamcmd  ',
        force_install_dir: '  /srv/dst  ',
        cluster: '  Cluster_1  ',
        backup: '  /srv/backup  ',
        mod_download_path: '  /srv/mods  ',
        bin: 128,
        beta: 9,
      }),
    ).toEqual({
      steamcmd: '/opt/steamcmd',
      force_install_dir: '/srv/dst',
      donot_starve_server_directory: '',
      cluster: 'Cluster_1',
      backup: '/srv/backup',
      mod_download_path: '/srv/mods',
      bin: 32,
      beta: 0,
      ugc_directory: '',
      persistent_storage_root: '',
      conf_dir: '',
    })
  })

  it('prepares save payload by trimming strings without changing valid toggles', () => {
    expect(
      prepareDstConfigForSave({
        steamcmd: '  /opt/steamcmd  ',
        force_install_dir: '  /srv/dst  ',
        donot_starve_server_directory: '  /dst/bin  ',
        cluster: '  Cluster_1  ',
        backup: '  /srv/backup  ',
        mod_download_path: '  /srv/mods  ',
        bin: 64,
        beta: 1,
        ugc_directory: '  /srv/ugc  ',
        persistent_storage_root: '  /srv/klei  ',
        conf_dir: '  DoNotStarveTogether  ',
      }),
    ).toEqual({
      steamcmd: '/opt/steamcmd',
      force_install_dir: '/srv/dst',
      donot_starve_server_directory: '/dst/bin',
      cluster: 'Cluster_1',
      backup: '/srv/backup',
      mod_download_path: '/srv/mods',
      bin: 64,
      beta: 1,
      ugc_directory: '/srv/ugc',
      persistent_storage_root: '/srv/klei',
      conf_dir: 'DoNotStarveTogether',
    })
  })

  it('returns the first Chinese validation error for missing required paths', () => {
    const config = createEmptyDstConfig()

    expect(validateDstConfig(config)).toBe('请填写 SteamCMD 目录')
  })

  it('validates bin and beta instead of silently coercing submitted values', () => {
    expect(
      validateDstConfig({
        ...createEmptyDstConfig(),
        steamcmd: '/opt/steamcmd',
        force_install_dir: '/srv/dst',
        cluster: 'Cluster_1',
        backup: '/srv/backup',
        mod_download_path: '/srv/mods',
        bin: 128,
      }),
    ).toBe('运行位数必须是 32 或 64')

    expect(
      validateDstConfig({
        ...createEmptyDstConfig(),
        steamcmd: '/opt/steamcmd',
        force_install_dir: '/srv/dst',
        cluster: 'Cluster_1',
        backup: '/srv/backup',
        mod_download_path: '/srv/mods',
        beta: 9,
      }),
    ).toBe('测试分支必须是关闭或开启')
  })
})
