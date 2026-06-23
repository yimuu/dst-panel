export interface UserProfile {
  ID?: number
  id?: number
  username?: string
  name?: string
  role?: string
  avatar?: string
  token?: string
  createdAt?: string
  updatedAt?: string
  [key: string]: unknown
}

export interface LoginRequest {
  username: string
  password: string
  [key: string]: unknown
}

export interface InitRequest {
  username: string
  password: string
  confirmPassword?: string
  [key: string]: unknown
}

export interface ClusterSummary {
  ID?: number
  id?: number
  clusterName?: string
  description?: string
  steamcmd?: string
  force_install_dir?: string
  backup?: string
  mod_download_path?: string
  uuid?: string
  beta?: number | boolean
  bin?: number
  ugc_directory?: string
  persistent_storage_root?: string
  conf_dir?: string
  createdAt?: string
  updatedAt?: string
  [key: string]: unknown
}

export interface LevelSummary {
  levelName?: string
  name?: string
  uuid?: string
  is_master?: boolean
  status?: boolean
  Ps?: Record<string, unknown>
  server_ini?: Record<string, unknown>
  leveldataoverride?: string
  modoverrides?: string
  [key: string]: unknown
}

export interface BackupFile {
  fileName?: string
  name?: string
  fileSize?: number
  size?: number
  createTime?: string
  time?: number
  [key: string]: unknown
}

export interface ModSummary {
  ID?: number
  id?: number
  modid?: string
  name?: string
  description?: string
  img?: string
  auth?: string
  file_url?: string
  last_time?: number
  mod_config?: string
  v?: string
  update?: boolean
  enabled?: boolean
  [key: string]: unknown
}

export interface TaskSummary {
  ID?: number
  id?: number
  clusterName?: string
  levelName?: string
  uuid?: string
  cron?: string
  category?: string
  comment?: string
  announcement?: string
  sleep?: number
  times?: number
  script?: number | boolean
  createdAt?: string
  updatedAt?: string
  [key: string]: unknown
}
