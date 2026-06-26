export function normalizeMapLevelName(levelName?: string): string {
  const trimmed = levelName?.trim()

  return trimmed || 'Master'
}

export function createMapCacheKey(timestamp = Date.now()): string {
  return String(timestamp)
}
