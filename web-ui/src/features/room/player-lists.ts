export type PlayerListKind = 'adminlist' | 'whitelist' | 'blacklist'

export function getPlayerListTitle(kind: PlayerListKind): string {
  return {
    adminlist: '管理员列表',
    whitelist: '白名单列表',
    blacklist: '黑名单列表',
  }[kind]
}
