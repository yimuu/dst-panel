export interface UserProfile {
  ID?: number
  id?: number
  username?: string
  displayName?: string
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
  userInfo: {
    username: string
    password: string
    displayName: string
    photoURL: string
  }
  dstConfig?: Record<string, unknown>
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

export interface ServerIniPayload {
  server_port: number
  is_master: boolean
  name: string
  id: number
  encode_user_path: boolean
  authentication_port: number
  master_server_port: number
}

export interface LevelSummary {
  levelName?: string
  name?: string
  uuid?: string
  is_master?: boolean
  status?: boolean
  Ps?: Record<string, unknown>
  server_ini?: ServerIniPayload
  leveldataoverride?: string
  modoverrides?: string
  [key: string]: unknown
}

export interface ClusterIni {
  game_mode: string
  max_players: number
  pvp: boolean
  pause_when_nobody: boolean
  vote_enabled: boolean
  vote_kick_enabled: boolean
  lan_only_cluster: boolean
  cluster_intention: string
  cluster_description: string
  cluster_password: string
  cluster_name: string
  offline_cluster: boolean
  cluster_language: string
  whitelist_slots: number
  tick_rate: number
  console_enabled: boolean
  max_snapshots: number
  shard_enabled: boolean
  bind_ip: string
  master_ip: string
  master_port: number
  cluster_key: string
  steam_group_id: string
  steam_group_only: boolean
  steam_group_admins: boolean
}

export interface ClusterIniEnvelope {
  cluster: ClusterIni
  token: string
}

export interface GameConfig {
  clusterIntention: string
  clusterName: string
  clusterDescription: string
  gameMode: string
  pvp: boolean
  maxPlayers: number
  max_snapshots: number
  clusterPassword: string
  token: string
  masterMapData: string
  cavesMapData: string
  modData: string
  type: number
  pause_when_nobody: boolean
  vote_enabled: boolean
}

export interface BackupFile {
  fileName?: string
  name?: string
  fileSize?: number
  size?: number
  createTime?: string
  time?: number | string
  [key: string]: unknown
}

export interface ModSummary {
  ID?: number
  id?: string | number
  modid?: string
  workshop_id?: string | number
  workshopId?: string | number
  publishedfileid?: string | number
  consumer_id?: number
  consumer_appid?: number
  creator_appid?: number
  name?: string
  description?: string
  desc?: string
  img?: string
  auth?: string
  author?: string
  file_url?: string
  last_time?: number
  time?: number | string
  mod_config?: string | Record<string, unknown>
  v?: string
  update?: boolean
  enabled?: boolean
  [key: string]: unknown
}

export interface TaskSummary {
  ID?: number
  id?: number
  jobId?: number | string
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
