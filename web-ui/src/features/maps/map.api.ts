import { apiGet } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { WorldSettingsDefinition } from '@/features/worlds/world-settings-model'

export async function getWorldSettingsDefinition(): Promise<WorldSettingsDefinition> {
  try {
    return await apiGet<WorldSettingsDefinition>('/api/dst-static/dst_world_setting.json')
  } catch {
    return apiGet<WorldSettingsDefinition>('/misc/dst_world_setting.json')
  }
}

export function getWorldgenCustomizationImageUrl(): string {
  return '/misc/worldgen_customization.webp'
}

export function getWorldSettingsCustomizationImageUrl(): string {
  return '/misc/worldsettings_customization.webp'
}

export function getMapImageUrl(levelName: string): string {
  return `/api/dst/map/image?${new URLSearchParams({ levelName }).toString()}`
}

export function generateMap(levelName: string): Promise<ApiEnvelope<unknown>> {
  return apiGet<ApiEnvelope<unknown>>(
    `/api/dst/map/gen?${new URLSearchParams({ levelName }).toString()}`,
  )
}
