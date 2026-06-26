import type { DstConfig } from './settings.api'

export interface PanelSettingsForm {
  panelName: string
  enableRegister: boolean
  steamApiKey: string
}

export function normalizePanelSettings(form: PanelSettingsForm): PanelSettingsForm {
  return {
    panelName: form.panelName.trim(),
    enableRegister: form.enableRegister,
    steamApiKey: form.steamApiKey.trim(),
  }
}

export function createEmptyDstConfig(): DstConfig {
  return {
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
  }
}

export function normalizeDstConfig(config: Partial<DstConfig>): DstConfig {
  const defaults = createEmptyDstConfig()

  return {
    steamcmd: readString(config.steamcmd, defaults.steamcmd).trim(),
    force_install_dir: readString(config.force_install_dir, defaults.force_install_dir).trim(),
    donot_starve_server_directory: readString(
      config.donot_starve_server_directory,
      defaults.donot_starve_server_directory,
    ).trim(),
    cluster: readString(config.cluster, defaults.cluster).trim(),
    backup: readString(config.backup, defaults.backup).trim(),
    mod_download_path: readString(config.mod_download_path, defaults.mod_download_path).trim(),
    bin: readNumber(config.bin, defaults.bin) === 64 ? 64 : 32,
    beta: readNumber(config.beta, defaults.beta) === 1 ? 1 : 0,
    ugc_directory: readString(config.ugc_directory, defaults.ugc_directory).trim(),
    persistent_storage_root: readString(
      config.persistent_storage_root,
      defaults.persistent_storage_root,
    ).trim(),
    conf_dir: readString(config.conf_dir, defaults.conf_dir).trim(),
  }
}

export function prepareDstConfigForSave(config: Partial<DstConfig>): DstConfig {
  const defaults = createEmptyDstConfig()

  return {
    steamcmd: readString(config.steamcmd, defaults.steamcmd).trim(),
    force_install_dir: readString(config.force_install_dir, defaults.force_install_dir).trim(),
    donot_starve_server_directory: readString(
      config.donot_starve_server_directory,
      defaults.donot_starve_server_directory,
    ).trim(),
    cluster: readString(config.cluster, defaults.cluster).trim(),
    backup: readString(config.backup, defaults.backup).trim(),
    mod_download_path: readString(config.mod_download_path, defaults.mod_download_path).trim(),
    bin: readNumber(config.bin, defaults.bin),
    beta: readNumber(config.beta, defaults.beta),
    ugc_directory: readString(config.ugc_directory, defaults.ugc_directory).trim(),
    persistent_storage_root: readString(
      config.persistent_storage_root,
      defaults.persistent_storage_root,
    ).trim(),
    conf_dir: readString(config.conf_dir, defaults.conf_dir).trim(),
  }
}

export function validateDstConfig(config: DstConfig): string | null {
  if (!config.steamcmd.trim()) return '请填写 SteamCMD 目录'
  if (!config.force_install_dir.trim()) return '请填写游戏安装目录'
  if (!config.cluster.trim()) return '请填写集群名称'
  if (!config.backup.trim()) return '请填写备份目录'
  if (!config.mod_download_path.trim()) return '请填写模组下载目录'
  if (config.bin !== 32 && config.bin !== 64) return '运行位数必须是 32 或 64'
  if (config.beta !== 0 && config.beta !== 1) return '测试分支必须是关闭或开启'

  return null
}

function readString(value: unknown, fallback: string): string {
  return typeof value === 'string' ? value : fallback
}

function readNumber(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}
