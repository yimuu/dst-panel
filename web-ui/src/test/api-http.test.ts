import { afterEach, describe, expect, expectTypeOf, it, vi } from 'vitest'

import {
  getInitStatus,
  initialize,
  updateUser,
  type UpdateUserRequest,
} from '@/features/auth/auth.api'
import { createBackup } from '@/features/backups/backup.api'
import {
  createCluster,
  updateCluster,
  type CreateClusterRequest,
  type UpdateClusterRequest,
} from '@/features/clusters/cluster.api'
import { applyPreinstallTemplate, startLevel, stopLevel } from '@/features/game/game.api'
import { saveLevels } from '@/features/levels/level.api'
import {
  checkWalrusHutPlains,
  generateMap,
  getSessionFile,
} from '@/features/maps/map.api'
import { deleteMod, searchMods } from '@/features/mods/mod.api'
import {
  getClusterIni,
  getPlayerList,
  saveClusterIni,
  savePlayerList,
} from '@/features/room/room.api'
import {
  deleteTask,
  getAutoCheck,
  getGameConfig,
  saveAutoCheck,
  saveGameConfig,
  saveTask,
  type AutoCheckPayload,
  type AutoCheckSummary,
  type SaveTaskRequest,
} from '@/features/settings/settings.api'
import { getTopActive } from '@/features/statistics/statistics.api'
import type { ApiEnvelope, PageResult } from '@/shared/api/types'
import { http, isApiSuccess, normalizeApiError } from '@/shared/api/http'
import type {
  ClusterIniEnvelope,
  GameConfig,
  InitRequest,
  ModSummary,
} from '@/shared/types/domain'

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

  it('uses a Chinese fallback when no API error message exists', () => {
    expect(normalizeApiError({}).message).toBe('请求失败')
  })

  it('passes levelName query params without unsupported cluster headers', async () => {
    const get = vi.spyOn(http, 'get').mockResolvedValue(successResponse)

    await startLevel('Master')
    await stopLevel('Caves')

    expect(get).toHaveBeenNthCalledWith(1, '/api/game/8level/start', {
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

  it('saves the full level list with the backend body shape and no cluster header', async () => {
    const put = vi.spyOn(http, 'put').mockResolvedValue(successResponse)
    const levels = [{ levelName: 'Master' }]

    await saveLevels(levels)

    expect(put).toHaveBeenCalledWith('/api/cluster/level', { levels }, undefined)
  })

  it('posts the nested first-run initialization payload', async () => {
    const post = vi.spyOn(http, 'post').mockResolvedValue(successResponse)
    const payload: InitRequest = {
      userInfo: {
        username: 'admin',
        displayName: '管理员',
        photoURL: '',
        password: 'secret',
      },
    }

    await initialize(payload)

    expect(post).toHaveBeenCalledWith('/api/init', payload, undefined)
  })

  it('uses backend query parameter names for task, mod, and statistics wrappers', async () => {
    const get = vi.spyOn(http, 'get').mockResolvedValue(successResponse)
    const del = vi.spyOn(http, 'delete').mockResolvedValue(successResponse)

    await deleteTask('42')
    await searchMods({ text: 'geometric placement', page: 1, size: 20, lang: 'zh' })
    await getTopActive({ N: 10, startDate: '2026-01-01', endDate: '2026-01-31' })
    await getAutoCheck('UPDATE_GAME')

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
    expect(get).toHaveBeenNthCalledWith(3, '/api/auto/check2', {
      params: {
        checkType: 'UPDATE_GAME',
      },
    })
  })

  it('uses backend paths for game config, preinstall, room, and map wrappers', async () => {
    const get = vi.spyOn(http, 'get').mockResolvedValue(successResponse)
    const post = vi.spyOn(http, 'post').mockResolvedValue(successResponse)
    const gameConfig = {
      clusterIntention: 'cooperative',
      clusterName: '测试世界',
      clusterDescription: '测试描述',
      gameMode: 'survival',
      pvp: false,
      maxPlayers: 8,
      max_snapshots: 6,
      clusterPassword: '',
      token: 'server-token',
      masterMapData: 'return {}',
      cavesMapData: 'return {}',
      modData: 'return {}',
      type: 0,
      pause_when_nobody: true,
      vote_enabled: true,
    } satisfies GameConfig

    await getGameConfig()
    await saveGameConfig(gameConfig)
    await applyPreinstallTemplate('forest')
    await getClusterIni()
    await saveClusterIni({ cluster: {}, token: 'server-token' } as ClusterIniEnvelope)
    await getPlayerList('whitelist')
    await savePlayerList('adminlist', ['KU_admin'])
    await generateMap('Master')
    await checkWalrusHutPlains('Master')
    await getSessionFile('Caves')

    expect(get).toHaveBeenNthCalledWith(1, '/api/game/config', undefined)
    expect(post).toHaveBeenNthCalledWith(1, '/api/game/config', gameConfig, undefined)
    expect(get).toHaveBeenNthCalledWith(2, '/api/game/preinstall', {
      params: { name: 'forest' },
    })
    expect(get).toHaveBeenNthCalledWith(3, '/api/game/8level/clusterIni', undefined)
    expect(post).toHaveBeenNthCalledWith(
      2,
      '/api/game/8level/clusterIni',
      { cluster: {}, token: 'server-token' },
      undefined,
    )
    expect(get).toHaveBeenNthCalledWith(4, '/api/game/8level/whitelist', undefined)
    expect(post).toHaveBeenNthCalledWith(
      3,
      '/api/game/8level/adminilist',
      { adminList: ['KU_admin'] },
      undefined,
    )
    expect(get).toHaveBeenNthCalledWith(5, '/api/dst/map/gen', {
      params: { levelName: 'Master' },
    })
    expect(get).toHaveBeenNthCalledWith(6, '/api/dst/map/has/walrusHut/plains', {
      params: { levelName: 'Master' },
    })
    expect(get).toHaveBeenNthCalledWith(7, '/api/dst/map/session/file', {
      params: { levelName: 'Caves' },
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
    expectTypeOf<ReturnType<typeof getAutoCheck>>().toEqualTypeOf<
      Promise<ApiEnvelope<AutoCheckSummary[]>>
    >()
    expectTypeOf<ReturnType<typeof saveAutoCheck>>().toEqualTypeOf<
      Promise<ApiEnvelope<AutoCheckSummary>>
    >()
    expectTypeOf<ReturnType<typeof getInitStatus>>().toEqualTypeOf<
      Promise<ApiEnvelope<boolean | Record<string, unknown> | null>>
    >()
    expectTypeOf<ReturnType<typeof initialize>>().toEqualTypeOf<Promise<ApiEnvelope<null>>>()
    expectTypeOf<ReturnType<typeof getClusterIni>>().toEqualTypeOf<
      Promise<ApiEnvelope<ClusterIniEnvelope>>
    >()
    expectTypeOf<ReturnType<typeof saveClusterIni>>().toEqualTypeOf<
      Promise<ApiEnvelope<ClusterIniEnvelope>>
    >()
    expectTypeOf<ReturnType<typeof getPlayerList>>().toEqualTypeOf<
      Promise<ApiEnvelope<string[]>>
    >()
    expectTypeOf<ReturnType<typeof savePlayerList>>().toEqualTypeOf<Promise<ApiEnvelope<null>>>()
    expectTypeOf<ReturnType<typeof getGameConfig>>().toEqualTypeOf<
      Promise<ApiEnvelope<GameConfig>>
    >()
    expectTypeOf<ReturnType<typeof saveGameConfig>>().toEqualTypeOf<Promise<ApiEnvelope<null>>>()
    expectTypeOf<ReturnType<typeof applyPreinstallTemplate>>().toEqualTypeOf<
      Promise<ApiEnvelope<null>>
    >()
    expectTypeOf<ReturnType<typeof generateMap>>().toEqualTypeOf<Promise<ApiEnvelope<null>>>()
    expectTypeOf<ReturnType<typeof checkWalrusHutPlains>>().toEqualTypeOf<
      Promise<ApiEnvelope<boolean>>
    >()
    expectTypeOf<ReturnType<typeof getSessionFile>>().toEqualTypeOf<Promise<ApiEnvelope<string>>>()
    expectTypeOf<Parameters<typeof createCluster>[0]>().toEqualTypeOf<CreateClusterRequest>()
    expectTypeOf<Parameters<typeof updateCluster>[0]>().toEqualTypeOf<UpdateClusterRequest>()
    expectTypeOf<Parameters<typeof initialize>[0]>().toEqualTypeOf<InitRequest>()
    expectTypeOf<Parameters<typeof saveTask>[0]>().toEqualTypeOf<SaveTaskRequest>()
    expectTypeOf<Parameters<typeof saveAutoCheck>[0]>().toEqualTypeOf<AutoCheckPayload>()
    expectTypeOf<Parameters<typeof updateUser>[0]>().toEqualTypeOf<UpdateUserRequest>()
  })
})
