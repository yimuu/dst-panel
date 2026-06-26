import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  checkWalrusHutPlains,
  generateMap,
  getMapImageUrl,
  getSessionFile,
} from '@/features/maps/map.api'
import { createMapCacheKey, normalizeMapLevelName } from '@/features/maps/map-state'
import { http } from '@/shared/api/http'

const successResponse = { data: { code: 0, data: null } }

describe('map API wrappers', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('calls map endpoints with levelName query params', async () => {
    const get = vi.spyOn(http, 'get').mockResolvedValue(successResponse)

    await generateMap('Master')
    await checkWalrusHutPlains('Master')
    await getSessionFile('Caves')

    expect(get).toHaveBeenNthCalledWith(1, '/api/dst/map/gen', {
      params: { levelName: 'Master' },
    })
    expect(get).toHaveBeenNthCalledWith(2, '/api/dst/map/has/walrusHut/plains', {
      params: { levelName: 'Master' },
    })
    expect(get).toHaveBeenNthCalledWith(3, '/api/dst/map/session/file', {
      params: { levelName: 'Caves' },
    })
  })

  it('builds map image URLs for direct image loading', () => {
    expect(getMapImageUrl('Master', '123')).toBe('/api/dst/map/image?levelName=Master&t=123')
    expect(getMapImageUrl('Caves')).toBe('/api/dst/map/image?levelName=Caves')
  })

  it('normalizes level names and creates cache keys for image refreshes', () => {
    expect(normalizeMapLevelName(' Master ')).toBe('Master')
    expect(normalizeMapLevelName('')).toBe('Master')
    expect(createMapCacheKey(123456789)).toBe('123456789')
  })
})
