import type { DstConfig } from '@/shared/types/domain'

const requiredFields: Array<keyof DstConfig> = [
  'steamcmd',
  'force_install_dir',
  'backup',
  'mod_download_path',
  'cluster',
]

export function createDefaultDstConfig(): DstConfig {
  return {
    steamcmd: '',
    force_install_dir: '',
    backup: '',
    mod_download_path: '',
    cluster: 'Cluster1',
    persistent_storage_root: '',
    conf_dir: 'DoNotStarveTogether',
    ugc_directory: '',
    donot_starve_server_directory: '',
    bin: '32',
    beta: 0,
  }
}

export function normalizeDstConfig(config: Partial<DstConfig>): DstConfig {
  const defaults = createDefaultDstConfig()
  const bin = String(config.bin ?? defaults.bin) === '64' ? '64' : '32'
  return {
    ...defaults,
    ...config,
    bin,
    beta: Number(config.beta ?? defaults.beta) === 1 ? 1 : 0,
  }
}

export function serializeDstConfig(config: DstConfig): Record<string, unknown> {
  return {
    ...config,
    bin: Number(config.bin),
  }
}

export function validateDstConfig(config: DstConfig): string[] {
  return requiredFields.filter((field) => !String(config[field] ?? '').trim())
}
