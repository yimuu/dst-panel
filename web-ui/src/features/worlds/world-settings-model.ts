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

export function buildWorldSettingGroups(
  source: WorldSettingGroupSourceMap,
  values: Record<string, string> = {},
): WorldSettingGroup[] {
  return Object.entries(source)
    .map(([key, group]) => ({
      key,
      title: group.text,
      order: group.order,
      items: Object.entries(group.items).map(([itemKey, item]) => {
        const desc = group.desc ?? { default: '默认' }
        const normalizedValue = normalizeWorldOptionValue(values[itemKey] ?? item.value)
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
  return `/misc/${atlasName}.webp`
}

export function parseWorldLocation(leveldataoverride: string): 'forest' | 'cave' {
  return /location\s*=\s*["']cave["']/.test(leveldataoverride) ? 'cave' : 'forest'
}

export function parseWorldOverrideValues(leveldataoverride: string): Record<string, string> {
  const values: Record<string, string> = {}
  const overrides = extractOverridesTable(leveldataoverride)
  if (!overrides) {
    return values
  }

  for (const match of overrides.body.matchAll(
    /([A-Za-z_][A-Za-z0-9_]*)\s*=\s*["']([^"']*)["']/g,
  )) {
    values[match[1]] = match[2]
  }

  return values
}

export function updateWorldOverrideValue(
  leveldataoverride: string,
  key: string,
  value: string,
): string {
  const source = leveldataoverride.trim() ? leveldataoverride : 'return { overrides = {} }'
  const overrides = extractOverridesTable(source)
  const assignment = `${key} = "${escapeLuaString(value)}"`
  if (!overrides) {
    return source.replace(/\}\s*$/, `,\n  overrides = {\n    ${assignment},\n  },\n}`)
  }

  const body = overrides.body
  const assignmentPattern = new RegExp(`(\\b${escapeRegExp(key)}\\s*=\\s*)["'][^"']*["']`)
  if (assignmentPattern.test(body)) {
    return `${source.slice(0, overrides.start + 1)}${body.replace(
      assignmentPattern,
      `$1"${escapeLuaString(value)}"`,
    )}${source.slice(overrides.end)}`
  }

  const insertion = body.trim() ? `${body.trimEnd()}\n    ${assignment},\n  ` : `\n    ${assignment},\n  `
  return `${source.slice(0, overrides.start + 1)}${insertion}${source.slice(overrides.end)}`
}

function extractOverridesTable(source: string): { start: number; end: number; body: string } | null {
  const overridesIndex = source.search(/\boverrides\b/)
  if (overridesIndex < 0) {
    return null
  }

  const start = source.indexOf('{', overridesIndex)
  if (start < 0) {
    return null
  }

  let depth = 0
  let quote: string | null = null
  let escaped = false
  for (let index = start; index < source.length; index += 1) {
    const char = source[index]
    if (quote) {
      if (escaped) {
        escaped = false
      } else if (char === '\\') {
        escaped = true
      } else if (char === quote) {
        quote = null
      }
      continue
    }

    if (char === '"' || char === "'") {
      quote = char
      continue
    }
    if (char === '{') {
      depth += 1
    } else if (char === '}') {
      depth -= 1
      if (depth === 0) {
        return { start, end: index, body: source.slice(start + 1, index) }
      }
    }
  }

  return null
}

function escapeLuaString(value: string): string {
  return value.replace(/\\/g, '\\\\').replace(/"/g, '\\"')
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}
