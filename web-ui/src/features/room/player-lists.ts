export const playerListKinds = ['adminlist', 'whitelist', 'blacklist'] as const

export type PlayerListKind = (typeof playerListKinds)[number]

export type PlayerListBodyKey = 'adminList' | 'whitelist' | 'blacklist'

export interface PlayerListContract {
  path: string
  bodyKey: PlayerListBodyKey
  label: string
}

export type PlayerListPayload =
  | { adminList: string[] }
  | { whitelist: string[] }
  | { blacklist: string[] }

export const playerListContracts = {
  adminlist: {
    path: '/api/game/8level/adminilist',
    bodyKey: 'adminList',
    label: '管理员列表',
  },
  whitelist: {
    path: '/api/game/8level/whitelist',
    bodyKey: 'whitelist',
    label: '白名单',
  },
  blacklist: {
    path: '/api/game/8level/blacklist',
    bodyKey: 'blacklist',
    label: '黑名单',
  },
} as const satisfies Record<PlayerListKind, PlayerListContract>

export function buildPlayerListPayload(
  kind: PlayerListKind,
  values: string[],
): PlayerListPayload {
  const { bodyKey } = playerListContracts[kind]

  return {
    [bodyKey]: values,
  } as PlayerListPayload
}
