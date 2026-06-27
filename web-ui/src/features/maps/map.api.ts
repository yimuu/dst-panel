import { apiGet } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { WorldSettingsDefinition } from '@/features/worlds/world-settings-model'

export function getWorldSettingsDefinition(): Promise<WorldSettingsDefinition> {
  return apiGet<WorldSettingsDefinition>('/api/dst-static/dst_world_setting.json')
}

export function getWorldgenCustomizationImageUrl(): string {
  return '/api/dst-static/worldgen_customization.webp'
}

export function getWorldSettingsCustomizationImageUrl(): string {
  return '/api/dst-static/worldsettings_customization.webp'
}

export function getMapImageUrl(levelName: string): string {
  return `/api/dst/map/image?${new URLSearchParams({ levelName }).toString()}`
}

export function generateMap(levelName: string): Promise<ApiEnvelope<unknown>> {
  return apiGet<ApiEnvelope<unknown>>(
    `/api/dst/map/gen?${new URLSearchParams({ levelName }).toString()}`,
  )
}
