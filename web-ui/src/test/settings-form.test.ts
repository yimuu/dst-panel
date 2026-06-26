import { describe, expect, it } from 'vitest'

import {
  createEmptyDstConfig,
  normalizePanelSettings,
  normalizeDstConfig,
  prepareDstConfigForSave,
  validateDstConfig,
} from '@/features/settings/settings-form'

describe('settings form', () => {
  it('normalizes whitespace in panel settings fields', () => {
    expect(
      normalizePanelSettings({
        panelName: '  猎人面板  ',
        enableRegister: true,
        steamApiKey: '  key  ',
      }),
    ).toEqual({
      panelName: '猎人面板',
      enableRegister: true,
      steamApiKey: 'key',
    })
  })

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

  it('treats whitespace-only required fields as missing', () => {
    const baseConfig = {
      ...createEmptyDstConfig(),
      steamcmd: '/opt/steamcmd',
      force_install_dir: '/srv/dst',
      cluster: 'Cluster_1',
      backup: '/srv/backup',
      mod_download_path: '/srv/mods',
    }

    expect(validateDstConfig({ ...baseConfig, steamcmd: '   ' })).toBe('请填写 SteamCMD 目录')
    expect(validateDstConfig({ ...baseConfig, force_install_dir: '   ' })).toBe(
      '请填写游戏安装目录',
    )
    expect(validateDstConfig({ ...baseConfig, cluster: '   ' })).toBe('请填写集群名称')
    expect(validateDstConfig({ ...baseConfig, backup: '   ' })).toBe('请填写备份目录')
    expect(validateDstConfig({ ...baseConfig, mod_download_path: '   ' })).toBe(
      '请填写模组下载目录',
    )
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

  it('keeps invalid submitted bin and beta values long enough for validation to reject them', () => {
    const baseConfig = {
      steamcmd: '/opt/steamcmd',
      force_install_dir: '/srv/dst',
      donot_starve_server_directory: '',
      cluster: 'Cluster_1',
      backup: '/srv/backup',
      mod_download_path: '/srv/mods',
      ugc_directory: '',
      persistent_storage_root: '',
      conf_dir: '',
    }

    expect(
      validateDstConfig(
        prepareDstConfigForSave({
          ...baseConfig,
          bin: undefined,
          beta: 1,
        }),
      ),
    ).toBe('运行位数必须是 32 或 64')

    expect(
      validateDstConfig(
        prepareDstConfigForSave({
          ...baseConfig,
          bin: Number.NaN,
          beta: 1,
        }),
      ),
    ).toBe('运行位数必须是 32 或 64')

    expect(
      validateDstConfig(
        prepareDstConfigForSave({
          ...baseConfig,
          bin: '64' as unknown as number,
          beta: 1,
        }),
      ),
    ).toBe('运行位数必须是 32 或 64')

    expect(
      validateDstConfig(
        prepareDstConfigForSave({
          ...baseConfig,
          bin: 64,
          beta: undefined,
        }),
      ),
    ).toBe('测试分支必须是关闭或开启')

    expect(
      validateDstConfig(
        prepareDstConfigForSave({
          ...baseConfig,
          bin: 64,
          beta: Number.NaN,
        }),
      ),
    ).toBe('测试分支必须是关闭或开启')

    expect(
      validateDstConfig(
        prepareDstConfigForSave({
          ...baseConfig,
          bin: 64,
          beta: '1' as unknown as number,
        }),
      ),
    ).toBe('测试分支必须是关闭或开启')
  })
})
