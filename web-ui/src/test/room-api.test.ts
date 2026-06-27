import type { AxiosRequestConfig, AxiosResponse } from 'axios'
import { afterEach, describe, expect, it } from 'vitest'

import {
  addPlayerListEntries,
  getClusterIni,
  getPlayerList,
  removePlayerListEntries,
  saveClusterIni,
  savePlayerList,
  type ClusterIniEnvelope,
} from '@/features/room/room.api'
import { api } from '@/shared/api/http'

const originalAdapter = api.defaults.adapter

function mockApiResponse(data: unknown): AxiosResponse {
  return {
    data,
    status: 200,
    statusText: 'OK',
    headers: {},
    config: {},
  } as AxiosResponse
}

function requestAt(requests: AxiosRequestConfig[], index: number): AxiosRequestConfig {
  const request = requests[index]
  expect(request).toBeDefined()
  return request as AxiosRequestConfig
}

afterEach(() => {
  api.defaults.adapter = originalAdapter
})

describe('room api contract', () => {
  it('uses the backend cluster.ini envelope route and payload', async () => {
    const requests: AxiosRequestConfig[] = []
    api.defaults.adapter = async (config) => {
      requests.push(config)
      return mockApiResponse({ code: 200, msg: 'success', data: null })
    }
    const payload: ClusterIniEnvelope = {
      cluster: {
        game_mode: 'survival',
        max_players: 8,
        pvp: false,
        pause_when_nobody: true,
        vote_enabled: true,
        vote_kick_enabled: true,
        lan_only_cluster: false,
        cluster_intention: 'cooperative',
        cluster_description: '测试房间',
        cluster_password: '',
        cluster_name: 'huhuhu-test',
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

    await getClusterIni()
    await saveClusterIni(payload)

    expect(requestAt(requests, 0).url).toBe('/api/game/8level/clusterIni')
    expect(requestAt(requests, 1).url).toBe('/api/game/8level/clusterIni')
    expect(JSON.parse(requestAt(requests, 1).data as string)).toEqual(payload)
  })

  it('maps player list kinds to the legacy paths and request keys', async () => {
    const requests: AxiosRequestConfig[] = []
    api.defaults.adapter = async (config) => {
      requests.push(config)
      return mockApiResponse({ code: 200, msg: 'success', data: [] })
    }

    await getPlayerList('adminlist')
    await getPlayerList('whitelist')
    await getPlayerList('blacklist')
    await savePlayerList('adminlist', ['KU_ADMIN'])
    await savePlayerList('whitelist', ['KU_WHITE'])
    await savePlayerList('blacklist', ['KU_BLACK'])
    await addPlayerListEntries('adminlist', ['KU_ADMIN'])
    await removePlayerListEntries('blacklist', ['KU_BLACK'])

    expect(requestAt(requests, 0).url).toBe('/api/game/8level/adminilist')
    expect(requestAt(requests, 1).url).toBe('/api/game/8level/whitelist')
    expect(requestAt(requests, 2).url).toBe('/api/game/8level/blacklist')
    expect(JSON.parse(requestAt(requests, 3).data as string)).toEqual({
      adminList: ['KU_ADMIN'],
    })
    expect(JSON.parse(requestAt(requests, 4).data as string)).toEqual({
      whitelist: ['KU_WHITE'],
    })
    expect(JSON.parse(requestAt(requests, 5).data as string)).toEqual({
      blacklist: ['KU_BLACK'],
    })
    expect(requestAt(requests, 6).url).toBe('/api/game/player/adminlist')
    expect(requestAt(requests, 7).url).toBe('/api/game/player/blacklist')
    expect(requestAt(requests, 7).method).toBe('delete')
  })
})
