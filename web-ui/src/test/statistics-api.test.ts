import type { AxiosRequestConfig, AxiosResponse } from 'axios'
import { afterEach, describe, expect, it } from 'vitest'

import {
  getActiveUsers,
  getRoleRates,
  getTopActiveUsers,
  regenerateStatistics,
} from '@/features/statistics/statistics.api'
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

function decodedUrl(config: AxiosRequestConfig): string {
  return decodeURIComponent(config.url ?? '')
}

function requestAt(requests: AxiosRequestConfig[], index: number): AxiosRequestConfig {
  const request = requests[index]
  expect(request).toBeDefined()
  return request as AxiosRequestConfig
}

afterEach(() => {
  api.defaults.adapter = originalAdapter
})

describe('statistics api contract', () => {
  it('preserves active user query parameters and backend axis shape', async () => {
    const requests: AxiosRequestConfig[] = []
    api.defaults.adapter = async (config) => {
      requests.push(config)
      return mockApiResponse({ code: 200, msg: 'success', data: { x: [], y1: [], y2: [] } })
    }

    await getActiveUsers({
      unit: 'DAY',
      startDate: '2026-03-01T00:00:00.000Z',
      endDate: '2026-03-08T00:00:00.000Z',
    })

    expect(decodedUrl(requestAt(requests, 0))).toBe(
      '/api/statistics/active/user?unit=DAY&startDate=2026-03-01T00:00:00.000Z&endDate=2026-03-08T00:00:00.000Z',
    )
  })

  it('preserves role-rate and top-statistics date windows', async () => {
    const requests: AxiosRequestConfig[] = []
    api.defaults.adapter = async (config) => {
      requests.push(config)
      return mockApiResponse({ code: 200, msg: 'success', data: [] })
    }

    await getRoleRates({
      startDate: '2026-03-01T00:00:00.000Z',
      endDate: '2026-03-08T00:00:00.000Z',
    })
    await getTopActiveUsers({
      limit: 10,
      startDate: '2026-03-01T00:00:00.000Z',
      endDate: '2026-03-08T00:00:00.000Z',
    })
    await regenerateStatistics({ limit: 5 })

    expect(decodedUrl(requestAt(requests, 0))).toBe(
      '/api/statistics/rate/role?startDate=2026-03-01T00:00:00.000Z&endDate=2026-03-08T00:00:00.000Z',
    )
    expect(decodedUrl(requestAt(requests, 1))).toBe(
      '/api/statistics/top/active?N=10&startDate=2026-03-01T00:00:00.000Z&endDate=2026-03-08T00:00:00.000Z',
    )
    expect(decodedUrl(requestAt(requests, 2))).toBe('/api/statistics/regenerate?N=5')
  })
})
