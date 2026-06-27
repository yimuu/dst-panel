import type { AxiosRequestConfig, AxiosResponse } from 'axios'
import { afterEach, describe, expect, expectTypeOf, it } from 'vitest'

import { sendGameCommand, type GameCommandPayload, type SystemInfo } from '@/features/game/game.api'
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

afterEach(() => {
  api.defaults.adapter = originalAdapter
})

function requestAt(requests: AxiosRequestConfig[], index: number): AxiosRequestConfig {
  const request = requests[index]
  expect(request).toBeDefined()
  return request as AxiosRequestConfig
}

describe('game api contract', () => {
  it('requires levelName for level commands and sends the backend payload shape', async () => {
    expectTypeOf<GameCommandPayload>().toEqualTypeOf<{
      levelName: string
      command: string
    }>()

    const requests: AxiosRequestConfig[] = []
    api.defaults.adapter = async (config) => {
      requests.push(config)
      return mockApiResponse({ code: 200, msg: 'success', data: null })
    }

    await sendGameCommand({ levelName: '森林', command: 'c_save()' })

    const request = requestAt(requests, 0)
    expect(request.url).toBe('/api/game/8level/command')
    expect(JSON.parse(request.data as string)).toEqual({
      levelName: '森林',
      command: 'c_save()',
    })
  })

  it('models the system info response returned by the Rust backend', () => {
    const sample: SystemInfo = {
      host: {
        os: 'linux',
        hostname: 'dst-host',
        platform: 'ubuntu',
        kernelArch: 'x86_64',
      },
      cpu: {
        cores: 8,
        cpuPercent: [7.95],
        cpuUsedPercent: 7.95,
        cpuUsed: 7.95,
      },
      mem: {
        total: 7510,
        available: 4560,
        used: 2950,
        usedPercent: 39.29,
      },
      disk: {
        devices: [
          {
            device: '/dev/disk1',
            mountpoint: '/',
            fstype: 'apfs',
            opts: 'rw',
            total: 200000,
            usage: 11.65,
            inodesUsage: 0,
          },
        ],
      },
      panelMemUsage: 12111000,
      panelCpuUsage: 0.5,
    }

    expectTypeOf<SystemInfo>().toEqualTypeOf<{
      host: {
        os: string
        hostname: string
        platform: string
        kernelArch: string
      }
      cpu: {
        cores: number
        cpuPercent: number[]
        cpuUsedPercent: number
        cpuUsed: number
      }
      mem: {
        total: number
        available: number
        used: number
        usedPercent: number
      }
      disk: {
        devices: Array<{
          device: string
          mountpoint: string
          fstype: string
          opts: string
          total: number
          usage: number
          inodesUsage: number
        }>
      }
      panelMemUsage: number
      panelCpuUsage: number
    }>()
    expect(sample.host.kernelArch).toBe('x86_64')
  })
})
