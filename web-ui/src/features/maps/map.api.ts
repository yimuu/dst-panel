import { apiGet } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'

export function generateMap(levelName: string): Promise<ApiEnvelope<null>> {
  return apiGet('/api/dst/map/gen', {
    params: { levelName },
  })
}

export function getMapImageUrl(levelName: string, cacheKey = ''): string {
  const params = new URLSearchParams({ levelName })
  if (cacheKey) {
    params.set('t', cacheKey)
  }

  return `/api/dst/map/image?${params.toString()}`
}

export function checkWalrusHutPlains(levelName: string): Promise<ApiEnvelope<boolean>> {
  return apiGet('/api/dst/map/has/walrusHut/plains', {
    params: { levelName },
  })
}

export function getSessionFile(levelName: string): Promise<ApiEnvelope<string>> {
  return apiGet('/api/dst/map/session/file', {
    params: { levelName },
  })
}
