export interface ModSummary {
  enabled?: boolean
  modid?: string
  name?: string
  workshopId?: string
  [key: string]: unknown
}

export interface ModConfigOption {
  data?: unknown
  description?: string
  hover?: string
  [key: string]: unknown
}

export interface ModConfigEntry {
  default?: unknown
  hover?: string
  label?: string
  name?: string
  options?: ModConfigOption[]
  [key: string]: unknown
}

export function formatWorkshopId(value: string): string {
  return value.trim().replace(/^workshop-/i, '')
}

export function isModEnabled(mod: ModSummary): boolean {
  return mod.enabled === true
}

export function getModDisplayName(mod: ModSummary): string {
  const name = typeof mod.name === 'string' ? mod.name.trim() : ''
  if (name) {
    return name
  }

  const modid = typeof mod.modid === 'string' ? mod.modid : ''
  const workshopId = typeof mod.workshopId === 'string' ? mod.workshopId : ''
  return formatWorkshopId(modid || workshopId || '未知模组')
}

export function getModWorkshopId(mod: ModSummary): string {
  const id = typeof mod.modid === 'string' ? mod.modid : mod.workshopId
  return formatWorkshopId(typeof id === 'string' ? id : '')
}

export function normalizeModConfig(value: unknown): ModConfigEntry[] {
  if (Array.isArray(value)) {
    return value.filter(isModConfigEntry)
  }

  if (typeof value === 'string' && value.trim()) {
    try {
      return normalizeModConfig(JSON.parse(value))
    } catch {
      return []
    }
  }

  if (isRecord(value)) {
    if ('configuration_options' in value) {
      return normalizeModConfig(value.configuration_options)
    }

    return Object.values(value).filter(isModConfigEntry)
  }

  return []
}

export function getModImageUrl(mod: ModSummary): string {
  const img = typeof mod.img === 'string' ? mod.img.trim() : ''
  if (img && img !== 'xxx') {
    return img
  }

  return '/assets/dst/mods.png'
}

export function createModOverridesLua(mods: ModSummary[]): string {
  const entries = mods
    .filter((mod) => isModEnabled(mod))
    .map(createModOverrideEntry)
    .filter((entry) => entry.length > 0)

  if (entries.length === 0) {
    return 'return {}'
  }

  return ['return {', ...entries, '}'].join('\n')
}

function createModOverrideEntry(mod: ModSummary): string {
  const modId = getModWorkshopId(mod)
  if (!modId) {
    return ''
  }

  const options = normalizeModConfig(mod.mod_config)
    .map((entry) => {
      const name = typeof entry.name === 'string' ? entry.name.trim() : ''
      if (!name) {
        return ''
      }
      return `      [${luaString(name)}] = ${luaValue(entry.default)},`
    })
    .filter((line) => line.length > 0)

  if (options.length === 0) {
    return `  ["workshop-${modId}"] = { enabled = true },`
  }

  return [
    `  ["workshop-${modId}"] = {`,
    '    enabled = true,',
    '    configuration_options = {',
    ...options,
    '    },',
    '  },',
  ].join('\n')
}

function luaValue(value: unknown): string {
  if (typeof value === 'boolean') {
    return value ? 'true' : 'false'
  }

  if (typeof value === 'number' && Number.isFinite(value)) {
    return String(value)
  }

  if (typeof value === 'string') {
    return luaString(value)
  }

  return luaString(String(value ?? ''))
}

function luaString(value: string): string {
  return JSON.stringify(value)
}

export function formatModUpdatedAt(timestamp: number | string | undefined): string {
  const numericTimestamp =
    typeof timestamp === 'string' ? Number.parseFloat(timestamp) : Number(timestamp ?? 0)
  if (!Number.isFinite(numericTimestamp) || numericTimestamp <= 0) {
    return '-'
  }

  const date = new Date(numericTimestamp * 1000)
  const datePart = `${date.getFullYear()}-${date.getMonth() + 1}-${date.getDate()}`
  const timePart = `${date.getHours()}:${padTime(date.getMinutes())}:${padTime(date.getSeconds())}`
  return `${datePart} ${timePart}`
}

export function getConfigEntryLabel(entry: ModConfigEntry): string {
  const label = typeof entry.label === 'string' ? entry.label.trim() : ''
  if (label) {
    return label
  }

  const name = typeof entry.name === 'string' ? entry.name.trim() : ''
  return name || '未命名选项'
}

export function getConfigOptionLabel(option: ModConfigOption): string {
  const description = typeof option.description === 'string' ? option.description.trim() : ''
  if (description) {
    return description
  }

  return stringifyConfigValue(option.data)
}

export function stringifyConfigValue(value: unknown): string {
  if (typeof value === 'string') {
    return value
  }

  if (typeof value === 'boolean') {
    return value ? 'Enabled' : 'Disabled'
  }

  if (value === undefined || value === null) {
    return '默认'
  }

  return String(value)
}

function padTime(value: number): string {
  return value.toString().padStart(2, '0')
}

function isModConfigEntry(value: unknown): value is ModConfigEntry {
  return isRecord(value) && !Array.isArray(value)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}
