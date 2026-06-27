export interface WorldSettingAtlas {
  name: string
  width: number
  height: number
  item_size: number
}

export interface WorldSettingItemSource {
  text: string
  value?: string
  image?: {
    x: number
    y: number
  }
}

export interface WorldSettingGroupSource {
  order: number
  text: string
  atlas: WorldSettingAtlas
  desc?: Record<string, string> | null
  items: Record<string, WorldSettingItemSource>
}

export type WorldSettingGroupSourceMap = Record<string, WorldSettingGroupSource>

export interface WorldSettingOption {
  label: string
  value: string
}

export interface WorldSettingItem {
  key: string
  label: string
  value: string
  atlasName: string
  atlas: WorldSettingAtlas
  image?: {
    x: number
    y: number
  }
  options: WorldSettingOption[]
}

export interface WorldSettingGroup {
  key: string
  title: string
  order: number
  items: WorldSettingItem[]
}

export interface WorldSettingsDefinition {
  zh: {
    forest: Record<string, WorldSettingGroupSourceMap>
    cave?: Record<string, WorldSettingGroupSourceMap>
  }
}

export function normalizeWorldOptionValue(value: string | undefined): string {
  return value && value.trim() ? value : 'default'
}

export function buildWorldSettingGroups(source: WorldSettingGroupSourceMap): WorldSettingGroup[] {
  return Object.entries(source)
    .map(([key, group]) => ({
      key,
      title: group.text,
      order: group.order,
      items: Object.entries(group.items).map(([itemKey, item]) => {
        const desc = group.desc ?? { default: '默认' }
        const normalizedValue = normalizeWorldOptionValue(item.value)
        return {
          key: itemKey,
          label: item.text,
          value: desc[normalizedValue] ? normalizedValue : 'default',
          atlasName: group.atlas.name,
          atlas: group.atlas,
          image: item.image,
          options: Object.entries(desc).map(([value, label]) => ({ label, value })),
        }
      }),
    }))
    .sort((left, right) => left.order - right.order)
}

export function getAtlasImageUrl(atlasName: string): string {
  return `/api/dst-static/${atlasName}.webp`
}
