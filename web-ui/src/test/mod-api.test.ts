import type { AxiosRequestConfig, AxiosResponse } from 'axios'
import { afterEach, describe, expect, it } from 'vitest'

import {
  deleteMod,
  deleteUgcMod,
  getMods,
  getUgcMods,
  saveModInfo,
  searchMods,
  subscribeMod,
  updateAllModInfo,
  updateMod,
  uploadModInfoFile,
  type ModInfoRecord,
} from '@/features/mods/mod.api'
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

describe('mod api contracts', () => {
  it('uses backend mod metadata routes and payloads', async () => {
    const requests: AxiosRequestConfig[] = []
    api.defaults.adapter = async (config) => {
      requests.push(config)
      return mockApiResponse({ code: 200, msg: 'success', data: null })
    }
    const record: ModInfoRecord = {
      ID: 1,
      auth: 'https://steamcommunity.com/id/rezecib',
      consumer_appid: 322330,
      creator_appid: 322330,
      description: 'shows player arrows',
      file_url: '',
      img: '/global.jpg',
      last_time: 1712828023,
      mod_config: '[]',
      modid: '378160973',
      name: 'Global Positions',
      update: false,
      v: '1.7.5',
    }

    await getMods()
    await saveModInfo(record)
    await updateAllModInfo()
    await subscribeMod('workshop-378160973')
    await updateMod('378160973')
    await deleteMod('378160973')

    expect(requestAt(requests, 0).url).toBe('/api/mod')
    expect(requestAt(requests, 0).method).toBe('get')
    expect(requestAt(requests, 1).url).toBe('/api/mod/modinfo')
    expect(requestAt(requests, 1).method).toBe('post')
    expect(JSON.parse(requestAt(requests, 1).data as string)).toEqual(record)
    expect(requestAt(requests, 2).url).toBe('/api/mod/modinfo?lang=zh')
    expect(requestAt(requests, 2).method).toBe('put')
    expect(requestAt(requests, 3).url).toBe('/api/mod/378160973?lang=zh')
    expect(requestAt(requests, 3).method).toBe('get')
    expect(requestAt(requests, 4).url).toBe('/api/mod/378160973?lang=zh')
    expect(requestAt(requests, 4).method).toBe('put')
    expect(requestAt(requests, 5).url).toBe('/api/mod/378160973')
    expect(requestAt(requests, 5).method).toBe('delete')
  })

  it('uses search, manual modinfo, and ugc acf routes', async () => {
    const requests: AxiosRequestConfig[] = []
    api.defaults.adapter = async (config) => {
      requests.push(config)
      return mockApiResponse({ code: 200, msg: 'success', data: [] })
    }

    await searchMods('global positions')
    await uploadModInfoFile({ workshopId: '378160973', modinfo: 'return {}' })
    await getUgcMods('Master')
    await deleteUgcMod('Master', 'workshop-378160973')

    expect(requestAt(requests, 0).url).toBe(
      '/api/mod/search?text=global+positions&page=1&size=10&lang=zh',
    )
    expect(requestAt(requests, 0).method).toBe('get')
    expect(requestAt(requests, 1).url).toBe('/api/mod/modinfo/file?lang=zh')
    expect(requestAt(requests, 1).method).toBe('post')
    expect(JSON.parse(requestAt(requests, 1).data as string)).toEqual({
      workshopId: '378160973',
      modinfo: 'return {}',
    })
    expect(requestAt(requests, 2).url).toBe('/api/mod/ugc/acf?levelName=Master')
    expect(requestAt(requests, 2).method).toBe('get')
    expect(requestAt(requests, 3).url).toBe('/api/mod/ugc?levelName=Master&workshopId=378160973')
    expect(requestAt(requests, 3).method).toBe('delete')
  })
})
