import { afterEach, describe, expect, expectTypeOf, it, vi } from 'vitest'

import { getInitStatus, updateUser } from '@/features/auth/auth.api'
import { createBackup } from '@/features/backups/backup.api'
import { createCluster, updateCluster } from '@/features/clusters/cluster.api'
import { startLevel, stopLevel } from '@/features/game/game.api'
import { saveLevels } from '@/features/levels/level.api'
import { deleteMod, searchMods } from '@/features/mods/mod.api'
import { deleteTask, saveTask } from '@/features/settings/settings.api'
import { getTopActive } from '@/features/statistics/statistics.api'
import type { ApiEnvelope, PageResult } from '@/shared/api/types'
import { http, isApiSuccess, normalizeApiError, withCluster } from '@/shared/api/http'
import type { ModSummary } from '@/shared/types/domain'

const successResponse = { data: { code: 0, data: null } }

describe('API HTTP helpers', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('treats backend success codes as successful envelopes', () => {
    const zeroCode: ApiEnvelope<null> = { code: 0, data: null }
    const okCode: ApiEnvelope<null> = { code: 200, data: null }

    expect(isApiSuccess(zeroCode)).toBe(true)
    expect(isApiSuccess(okCode)).toBe(true)
  })

  it('normalizes Axios-like response errors', () => {
    const error = {
      response: {
        status: 401,
        data: {
          code: 401,
          msg: 'unauthorized',
        },
      },
    }

    expect(normalizeApiError(error)).toEqual({
      status: 401,
      code: 401,
      message: 'unauthorized',
      details: {
        code: 401,
        msg: 'unauthorized',
      },
    })
  })

  it('falls back to backend message fields and plain error messages', () => {
    expect(
      normalizeApiError({
        response: {
          data: {
            message: 'backend message',
          },
        },
      }).message,
    ).toBe('backend message')

    expect(normalizeApiError(new Error('plain failure')).message).toBe('plain failure')
  })

  it('builds the optional cluster header config', () => {
    expect(withCluster()).toBeUndefined()
    expect(withCluster('Cluster1')).toEqual({
      headers: {
        Cluster: 'Cluster1',
      },
    })
  })

  it('passes levelName query params to game lifecycle wrappers', async () => {
    const get = vi.spyOn(http, 'get').mockResolvedValue(successResponse)

    await startLevel('Master', 'Cluster1')
    await stopLevel('Caves')

    expect(get).toHaveBeenNthCalledWith(1, '/api/game/8level/start', {
      headers: {
        Cluster: 'Cluster1',
      },
      params: {
        levelName: 'Master',
      },
    })
    expect(get).toHaveBeenNthCalledWith(2, '/api/game/8level/stop', {
      params: {
        levelName: 'Caves',
      },
    })
  })

  it('saves the full level list with the backend body shape', async () => {
    const put = vi.spyOn(http, 'put').mockResolvedValue(successResponse)
    const levels = [{ levelName: 'Master' }]

    await saveLevels(levels, 'Cluster1')

    expect(put).toHaveBeenCalledWith(
      '/api/cluster/level',
      { levels },
      {
        headers: {
          Cluster: 'Cluster1',
        },
      },
    )
  })

  it('uses backend query parameter names for task, mod, and statistics wrappers', async () => {
    const get = vi.spyOn(http, 'get').mockResolvedValue(successResponse)
    const del = vi.spyOn(http, 'delete').mockResolvedValue(successResponse)

    await deleteTask('42')
    await searchMods({ text: 'geometric placement', page: 1, size: 20, lang: 'zh' })
    await getTopActive({ N: 10, startDate: '2026-01-01', endDate: '2026-01-31' })

    expect(del).toHaveBeenCalledWith('/api/task', {
      params: {
        jobId: '42',
      },
    })
    expect(get).toHaveBeenNthCalledWith(1, '/api/mod/search', {
      params: {
        text: 'geometric placement',
        page: 1,
        size: 20,
        lang: 'zh',
      },
    })
    expect(get).toHaveBeenNthCalledWith(2, '/api/statistics/top/active', {
      params: {
        N: 10,
        startDate: '2026-01-01',
        endDate: '2026-01-31',
      },
    })
  })

  it('keeps important wrapper response types aligned with backend envelopes', () => {
    expectTypeOf<ReturnType<typeof searchMods>>().toEqualTypeOf<
      Promise<ApiEnvelope<PageResult<ModSummary>>>
    >()
    expectTypeOf<ReturnType<typeof createCluster>>().toEqualTypeOf<Promise<ApiEnvelope<null>>>()
    expectTypeOf<ReturnType<typeof updateCluster>>().toEqualTypeOf<Promise<ApiEnvelope<null>>>()
    expectTypeOf<ReturnType<typeof saveTask>>().toEqualTypeOf<Promise<ApiEnvelope<null>>>()
    expectTypeOf<ReturnType<typeof updateUser>>().toEqualTypeOf<Promise<ApiEnvelope<null>>>()
    expectTypeOf<ReturnType<typeof createBackup>>().toEqualTypeOf<Promise<ApiEnvelope<null>>>()
    expectTypeOf<ReturnType<typeof deleteMod>>().toEqualTypeOf<Promise<ApiEnvelope<string>>>()
    expectTypeOf<ReturnType<typeof getInitStatus>>().toEqualTypeOf<
      Promise<ApiEnvelope<boolean | Record<string, unknown> | null>>
    >()
  })
})
