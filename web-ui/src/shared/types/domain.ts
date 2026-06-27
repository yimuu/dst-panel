export interface CurrentUser {
  id?: string | number
  ID?: string | number
  username?: string
  name?: string
  displayName?: string
  photoURL?: string
  role?: string
  createdAt?: string
  created_at?: string
}

export interface LevelSummary {
  uuid?: string
  name?: string
  levelName?: string
  is_master?: boolean
  status?: boolean
  [key: string]: unknown
}

export interface DstConfig {
  steamcmd: string
  force_install_dir: string
  backup: string
  mod_download_path: string
  cluster: string
  persistent_storage_root: string
  conf_dir: string
  ugc_directory: string
  donot_starve_server_directory: string
  bin: '32' | '64'
  beta: 0 | 1
}
