import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  getClusterIni,
  getPlayerList,
  saveClusterIni,
  savePlayerList,
} from '@/features/room/room.api'
import { http } from '@/shared/api/http'
import type { ClusterIniEnvelope } from '@/shared/types/domain'

const successResponse = { data: { code: 0, data: null } }

function createClusterIniPayload(): ClusterIniEnvelope {
  return {
    cluster: {
      game_mode: 'survival',
      max_players: 8,
      pvp: false,
      pause_when_nobody: true,
      vote_enabled: true,
      vote_kick_enabled: true,
      lan_only_cluster: false,
      cluster_intention: 'cooperative',
      cluster_description: '测试世界描述',
      cluster_password: '',
      cluster_name: '测试世界',
      offline_cluster: false,
      cluster_language: 'zh',
      whitelist_slots: 0,
      tick_rate: 15,
      console_enabled: true,
      max_snapshots: 6,
      shard_enabled: true,
      bind_ip: '0.0.0.0',
      master_ip: '127.0.0.1',
      master_port: 10888,
      cluster_key: '',
      steam_group_id: '',
      steam_group_only: false,
      steam_group_admins: false,
    },
    token: 'server-token',
  }
}

describe('room API wrappers', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('loads and saves cluster.ini with the backend envelope shape', async () => {
    const payload = createClusterIniPayload()
    const get = vi.spyOn(http, 'get').mockResolvedValue({ data: { code: 0, data: payload } })
    const post = vi.spyOn(http, 'post').mockResolvedValue({ data: { code: 0, data: payload } })

    await getClusterIni()
    await saveClusterIni(payload)

    expect(get).toHaveBeenCalledWith('/api/game/8level/clusterIni', undefined)
    expect(post).toHaveBeenCalledWith('/api/game/8level/clusterIni', payload, undefined)
  })

  it('uses the legacy player-list endpoints and payload keys', async () => {
    const get = vi.spyOn(http, 'get').mockResolvedValue({ data: { code: 0, data: ['KU_admin'] } })
    const post = vi.spyOn(http, 'post').mockResolvedValue(successResponse)

    await getPlayerList('adminlist')
    await savePlayerList('whitelist', ['KU_abc'])
    await savePlayerList('blacklist', ['KU_block'])

    expect(get).toHaveBeenCalledWith('/api/game/8level/adminilist', undefined)
    expect(post).toHaveBeenNthCalledWith(
      1,
      '/api/game/8level/whitelist',
      { whitelist: ['KU_abc'] },
      undefined,
    )
    expect(post).toHaveBeenNthCalledWith(
      2,
      '/api/game/8level/blacklist',
      { blacklist: ['KU_block'] },
      undefined,
    )
  })
})
