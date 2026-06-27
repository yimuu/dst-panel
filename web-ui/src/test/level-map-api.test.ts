import type { AxiosRequestConfig, AxiosResponse } from 'axios'
import { afterEach, describe, expect, it } from 'vitest'

import {
  createLevel,
  deleteLevel,
  getLevels,
  saveLevels,
  type WorldLevel,
} from '@/features/levels/level.api'
import {
  generateMap,
  getMapImageUrl,
  getWorldSettingsCustomizationImageUrl,
  getWorldSettingsDefinition,
  getWorldgenCustomizationImageUrl,
} from '@/features/maps/map.api'
import { applyPreinstall } from '@/features/maps/map-state'
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

describe('level and map api contracts', () => {
  it('uses the backend level routes and payload shape', async () => {
    const requests: AxiosRequestConfig[] = []
    api.defaults.adapter = async (config) => {
      requests.push(config)
      return mockApiResponse({ code: 200, msg: 'success', data: [] })
    }
    const world: WorldLevel = {
      levelName: '森林',
      uuid: 'Master',
      serverini: '[NETWORK]',
      leveldataoverride: 'return {}',
      modoverrides: 'return {}',
    }

    await getLevels()
    await saveLevels([world])
    await createLevel(world)
    await deleteLevel('Caves')

    expect(requestAt(requests, 0).url).toBe('/api/cluster/level')
    expect(requestAt(requests, 1).url).toBe('/api/cluster/level')
    expect(requestAt(requests, 1).method).toBe('put')
    expect(JSON.parse(requestAt(requests, 1).data as string)).toEqual({ levels: [world] })
    expect(requestAt(requests, 2).url).toBe('/api/cluster/level')
    expect(requestAt(requests, 2).method).toBe('post')
    expect(JSON.parse(requestAt(requests, 2).data as string)).toEqual(world)
    expect(requestAt(requests, 3).url).toBe('/api/cluster/level?levelName=Caves')
    expect(requestAt(requests, 3).method).toBe('delete')
  })

  it('uses dst-static and map routes with levelName query parameters', async () => {
    const requests: AxiosRequestConfig[] = []
    api.defaults.adapter = async (config) => {
      requests.push(config)
      return mockApiResponse({ code: 200, msg: 'success', data: null })
    }

    await getWorldSettingsDefinition()
    await generateMap('Master')
    await applyPreinstall('标准世界')

    expect(requestAt(requests, 0).url).toBe('/api/dst-static/dst_world_setting.json')
    expect(requestAt(requests, 1).url).toBe('/api/dst/map/gen?levelName=Master')
    expect(requestAt(requests, 2).url).toBe(
      '/api/game/preinstall?name=%E6%A0%87%E5%87%86%E4%B8%96%E7%95%8C',
    )
    expect(getMapImageUrl('Caves')).toBe('/api/dst/map/image?levelName=Caves')
    expect(getWorldgenCustomizationImageUrl()).toBe('/api/dst-static/worldgen_customization.webp')
    expect(getWorldSettingsCustomizationImageUrl()).toBe(
      '/api/dst-static/worldsettings_customization.webp',
    )
  })
})
