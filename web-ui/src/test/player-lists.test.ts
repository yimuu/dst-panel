import { describe, expect, it } from 'vitest'

import { getPlayerListTitle } from '@/features/room/player-lists'

describe('player list labels', () => {
  it('maps list kinds to official titles', () => {
    expect(getPlayerListTitle('adminlist')).toBe('管理员列表')
    expect(getPlayerListTitle('whitelist')).toBe('白名单列表')
    expect(getPlayerListTitle('blacklist')).toBe('黑名单列表')
  })
})
