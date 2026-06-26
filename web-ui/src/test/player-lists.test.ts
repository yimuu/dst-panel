import { describe, expect, it } from 'vitest'

import {
  buildPlayerListPayload,
  playerListContracts,
  playerListKinds,
} from '@/features/room/player-lists'

describe('player list contracts', () => {
  it('maps menu kinds to the exact backend endpoints', () => {
    expect(playerListKinds).toEqual(['adminlist', 'whitelist', 'blacklist'])
    expect(playerListContracts).toMatchObject({
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
    })
  })

  it('builds overwrite payloads with backend-compatible body keys', () => {
    expect(buildPlayerListPayload('adminlist', ['KU_admin'])).toEqual({
      adminList: ['KU_admin'],
    })
    expect(buildPlayerListPayload('whitelist', ['KU_white'])).toEqual({
      whitelist: ['KU_white'],
    })
    expect(buildPlayerListPayload('blacklist', ['KU_black'])).toEqual({
      blacklist: ['KU_black'],
    })
  })
})
