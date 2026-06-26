export type ApiMethod = 'GET' | 'POST' | 'PUT' | 'DELETE'

export interface ApiContract {
  method: ApiMethod
  path: string
  feature: 'auth' | 'backup' | 'cluster' | 'game' | 'level' | 'mod' | 'setting' | 'statistics'
}

export const apiContracts: ApiContract[] = [
  { method: 'GET', path: '/api/init', feature: 'auth' },
  { method: 'POST', path: '/api/init', feature: 'auth' },
  { method: 'POST', path: '/api/login', feature: 'auth' },
  { method: 'GET', path: '/api/logout', feature: 'auth' },
  { method: 'POST', path: '/api/change/password', feature: 'auth' },
  { method: 'GET', path: '/api/user', feature: 'auth' },
  { method: 'POST', path: '/api/user', feature: 'auth' },

  { method: 'GET', path: '/api/cluster', feature: 'cluster' },
  { method: 'POST', path: '/api/cluster', feature: 'cluster' },
  { method: 'PUT', path: '/api/cluster', feature: 'cluster' },
  { method: 'DELETE', path: '/api/cluster', feature: 'cluster' },

  { method: 'GET', path: '/api/cluster/level', feature: 'level' },
  { method: 'POST', path: '/api/cluster/level', feature: 'level' },
  { method: 'PUT', path: '/api/cluster/level', feature: 'level' },
  { method: 'DELETE', path: '/api/cluster/level', feature: 'level' },

  { method: 'GET', path: '/api/game/8level/status', feature: 'game' },
  { method: 'GET', path: '/api/game/8level/start', feature: 'game' },
  { method: 'GET', path: '/api/game/8level/stop', feature: 'game' },
  { method: 'POST', path: '/api/game/8level/command', feature: 'game' },
  { method: 'GET', path: '/api/game/system/info', feature: 'game' },

  { method: 'GET', path: '/api/game/backup', feature: 'backup' },
  { method: 'POST', path: '/api/game/backup', feature: 'backup' },
  { method: 'DELETE', path: '/api/game/backup', feature: 'backup' },
  { method: 'PUT', path: '/api/game/backup', feature: 'backup' },
  { method: 'GET', path: '/api/game/backup/download', feature: 'backup' },

  { method: 'GET', path: '/api/mod', feature: 'mod' },
  { method: 'GET', path: '/api/mod/search', feature: 'mod' },
  { method: 'GET', path: '/api/mod/:id', feature: 'mod' },
  { method: 'PUT', path: '/api/mod/:id', feature: 'mod' },
  { method: 'DELETE', path: '/api/mod/:id', feature: 'mod' },
  { method: 'POST', path: '/api/mod/modinfo', feature: 'mod' },
  { method: 'PUT', path: '/api/mod/modinfo', feature: 'mod' },

  { method: 'GET', path: '/api/dst/config', feature: 'setting' },
  { method: 'POST', path: '/api/dst/config', feature: 'setting' },
  { method: 'GET', path: '/api/dst/lobby/server/detail', feature: 'setting' },
  { method: 'GET', path: '/api/task', feature: 'setting' },
  { method: 'POST', path: '/api/task', feature: 'setting' },
  { method: 'DELETE', path: '/api/task', feature: 'setting' },
  { method: 'GET', path: '/api/auto/check2', feature: 'setting' },
  { method: 'POST', path: '/api/auto/check2', feature: 'setting' },

  { method: 'GET', path: '/api/statistics/active/user', feature: 'statistics' },
  { method: 'GET', path: '/api/statistics/top/active', feature: 'statistics' },
  { method: 'GET', path: '/api/statistics/rate/role', feature: 'statistics' },
  { method: 'GET', path: '/api/statistics/regenerate', feature: 'statistics' },
]
