import type { WorldLevel } from './level.api'

export const DEFAULT_SELECTOR_WORKSHOP_ID = 'workshop-1754389029'

export interface SelectorWorldConfig {
  id: string
  name: string
  category?: string
  note?: string
  desc?: string
  galleryful?: number
  extra?: boolean
  is_cave?: boolean
  invisible?: boolean
}

export function normalizeSelectorWorkshopId(value: string | undefined): string {
  const trimmed = (value ?? '').trim()
  if (!trimmed) {
    return DEFAULT_SELECTOR_WORKSHOP_ID
  }

  return trimmed.startsWith('workshop-') ? trimmed : `workshop-${trimmed}`
}

export function parseSelectorWorldConfig(
  modoverrides: string,
  workshopId = DEFAULT_SELECTOR_WORKSHOP_ID,
): SelectorWorldConfig[] {
  const normalizedWorkshopId = normalizeSelectorWorkshopId(workshopId)
  const workshopTable = extractWorkshopTable(modoverrides, normalizedWorkshopId)
  const worldConfig = workshopTable ? extractAssignedTable(workshopTable.body, 'world_config') : null
  if (!worldConfig) {
    return []
  }

  return parseWorldConfigTable(worldConfig.body)
}

export function applySelectorWorldConfigToModoverrides(
  modoverrides: string,
  workshopId: string,
  configs: SelectorWorldConfig[],
): string {
  const normalizedWorkshopId = normalizeSelectorWorkshopId(workshopId)
  const normalizedConfig = configs
    .map((config) => sanitizeSelectorConfig(config))
    .filter((config) => config.id && config.name)
  const workshopBlock = formatSelectorWorkshopBlock(normalizedWorkshopId, normalizedConfig)
  const source = ensureLuaReturnTable(modoverrides)
  const existingWorkshop = extractWorkshopTable(source, normalizedWorkshopId)
  if (existingWorkshop) {
    return `${source.slice(0, existingWorkshop.assignmentStart)}${workshopBlock}${source.slice(
      existingWorkshop.assignmentEnd,
    )}`
  }

  const outer = extractFirstTable(source)
  if (!outer) {
    return `return {\n${indent(workshopBlock, 2)}\n}`
  }

  const beforeClose = source.slice(0, outer.end)
  const afterClose = source.slice(outer.end)
  const needsComma = /[}\]"'\w)]\s*$/.test(beforeClose.slice(0, -1))
  const separator = needsComma ? ',\n' : '\n'

  return `${beforeClose}${separator}${indent(workshopBlock, 2)}\n${afterClose}`
}

export function worldConfigFromLevels(levels: WorldLevel[]): SelectorWorldConfig[] {
  return levels.map((level) => ({
    id: level.uuid,
    name: level.levelName || level.uuid,
    category: level.is_master ? '地上' : '洞穴',
    note: level.is_master ? '主世界' : '从世界',
    galleryful: 6,
    extra: false,
    is_cave: !level.is_master || /cave/i.test(level.uuid),
    invisible: false,
  }))
}

function sanitizeSelectorConfig(config: SelectorWorldConfig): SelectorWorldConfig {
  return {
    id: String(config.id ?? '').trim(),
    name: String(config.name ?? '').trim(),
    category: emptyToUndefined(config.category),
    note: emptyToUndefined(config.note),
    desc: emptyToUndefined(config.desc),
    galleryful:
      typeof config.galleryful === 'number' && Number.isFinite(config.galleryful)
        ? config.galleryful
        : undefined,
    extra: Boolean(config.extra),
    is_cave: Boolean(config.is_cave),
    invisible: Boolean(config.invisible),
  }
}

function emptyToUndefined(value: unknown): string | undefined {
  const normalized = String(value ?? '').trim()
  return normalized ? normalized : undefined
}

function ensureLuaReturnTable(source: string): string {
  const trimmed = source.trim()
  if (!trimmed || trimmed === 'return {}' || trimmed === 'return {  }') {
    return 'return {\n}'
  }
  if (trimmed.startsWith('return')) {
    return source
  }
  return `return {\n${source}\n}`
}

function formatSelectorWorkshopBlock(workshopId: string, configs: SelectorWorldConfig[]): string {
  return `["${escapeLuaString(workshopId)}"] = {
  configuration_options = {
    world_config = {
${configs.map((config) => indent(formatWorldConfigEntry(config), 6)).join(',\n')}
    },
    default_galleryful = 0,
    auto_balancing = true,
    no_bat = true,
    world_prompt = false,
    say_dest = true,
    migration_postern = false,
    ignore_sinkholes = false,
    open_button = true,
    migrator_required = false,
    force_population = false,
    name_button = true,
    always_show_ui = false,
    gift_toasts_offset = 100,
  },
  enabled = true,
}`
}

function formatWorldConfigEntry(config: SelectorWorldConfig): string {
  const lines = [
    `name = "${escapeLuaString(config.name)}"`,
    config.category ? `category = "${escapeLuaString(config.category)}"` : undefined,
    config.note ? `note = "${escapeLuaString(config.note)}"` : undefined,
    config.desc ? `desc = "${escapeLuaString(config.desc)}"` : undefined,
    typeof config.galleryful === 'number' ? `galleryful = ${config.galleryful}` : undefined,
    `extra = ${Boolean(config.extra)}`,
    `is_cave = ${Boolean(config.is_cave)}`,
    `invisible = ${Boolean(config.invisible)}`,
  ].filter(Boolean)

  return `["${escapeLuaString(config.id)}"] = {\n${indent(lines.join(',\n'), 2)}\n}`
}

function escapeLuaString(value: string): string {
  return value.replace(/\\/g, '\\\\').replace(/"/g, '\\"')
}

function parseWorldConfigTable(body: string): SelectorWorldConfig[] {
  const entries: SelectorWorldConfig[] = []
  let index = 0
  while (index < body.length) {
    const parsedKey = parseLuaKey(body, index)
    if (!parsedKey) {
      index += 1
      continue
    }
    index = parsedKey.end
    index = skipWhitespaceAndEquals(body, index)
    if (body[index] !== '{') {
      continue
    }
    const entryTable = extractTableAt(body, index)
    if (!entryTable) {
      break
    }
    const fields = parseLuaFields(entryTable.body)
    entries.push({
      id: parsedKey.key,
      name: String(fields.name ?? parsedKey.key),
      category: stringField(fields.category),
      note: stringField(fields.note),
      desc: stringField(fields.desc),
      galleryful: numberField(fields.galleryful),
      extra: booleanField(fields.extra),
      is_cave: booleanField(fields.is_cave),
      invisible: booleanField(fields.invisible),
    })
    index = entryTable.end + 1
  }

  return entries
}

function parseLuaFields(body: string): Record<string, unknown> {
  const fields: Record<string, unknown> = {}
  let index = 0
  while (index < body.length) {
    const parsedKey = parseLuaKey(body, index)
    if (!parsedKey) {
      index += 1
      continue
    }
    index = skipWhitespaceAndEquals(body, parsedKey.end)
    const parsedValue = parseLuaValue(body, index)
    if (!parsedValue) {
      continue
    }
    fields[parsedKey.key] = parsedValue.value
    index = parsedValue.end
  }

  return fields
}

function parseLuaKey(source: string, start: number): { key: string; end: number } | null {
  let index = skipWhitespace(source, start)
  if (source[index] === '[') {
    index = skipWhitespace(source, index + 1)
    const quote = source[index]
    if (quote !== '"' && quote !== "'") {
      return null
    }
    const parsed = readQuotedString(source, index)
    if (!parsed) {
      return null
    }
    index = skipWhitespace(source, parsed.end)
    if (source[index] !== ']') {
      return null
    }
    return { key: parsed.value, end: index + 1 }
  }

  const match = /^[A-Za-z_][A-Za-z0-9_]*/.exec(source.slice(index))
  if (!match) {
    return null
  }

  return { key: match[0], end: index + match[0].length }
}

function parseLuaValue(source: string, start: number): { value: unknown; end: number } | null {
  const index = skipWhitespace(source, start)
  const char = source[index]
  if (char === '"' || char === "'") {
    return readQuotedString(source, index)
  }

  const literal = /^(true|false)\b/.exec(source.slice(index))
  if (literal) {
    return { value: literal[1] === 'true', end: index + literal[1].length }
  }

  const number = /^-?\d+(?:\.\d+)?/.exec(source.slice(index))
  if (number) {
    return { value: Number(number[0]), end: index + number[0].length }
  }

  if (char === '{') {
    const table = extractTableAt(source, index)
    if (table) {
      return { value: table.body, end: table.end + 1 }
    }
  }

  return null
}

function stringField(value: unknown): string | undefined {
  return typeof value === 'string' && value ? value : undefined
}

function numberField(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

function booleanField(value: unknown): boolean | undefined {
  return typeof value === 'boolean' ? value : undefined
}

function extractWorkshopTable(source: string, workshopId: string) {
  const quotedKey = `["${workshopId}"]`
  let keyStart = source.indexOf(quotedKey)
  if (keyStart < 0) {
    keyStart = source.indexOf(`['${workshopId}']`)
  }
  if (keyStart < 0) {
    return null
  }

  const tableStart = source.indexOf('{', keyStart + workshopId.length)
  if (tableStart < 0) {
    return null
  }

  const table = extractTableAt(source, tableStart)
  if (!table) {
    return null
  }

  let assignmentEnd = table.end + 1
  while (/\s/.test(source[assignmentEnd] ?? '')) {
    assignmentEnd += 1
  }
  if (source[assignmentEnd] === ',') {
    assignmentEnd += 1
  }

  return {
    ...table,
    assignmentStart: keyStart,
    assignmentEnd,
  }
}

function extractAssignedTable(source: string, key: string) {
  const keyIndex = source.search(new RegExp(`\\b${key}\\b`))
  if (keyIndex < 0) {
    return null
  }

  const tableStart = source.indexOf('{', keyIndex + key.length)
  if (tableStart < 0) {
    return null
  }

  return extractTableAt(source, tableStart)
}

function extractFirstTable(source: string) {
  const tableStart = source.indexOf('{')
  return tableStart >= 0 ? extractTableAt(source, tableStart) : null
}

function extractTableAt(source: string, start: number): { body: string; start: number; end: number } | null {
  if (source[start] !== '{') {
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
        return {
          start,
          end: index,
          body: source.slice(start + 1, index),
        }
      }
    }
  }

  return null
}

function readQuotedString(source: string, start: number): { value: string; end: number } | null {
  const quote = source[start]
  let value = ''
  let escaped = false
  for (let index = start + 1; index < source.length; index += 1) {
    const char = source[index]
    if (escaped) {
      value += char
      escaped = false
    } else if (char === '\\') {
      escaped = true
    } else if (char === quote) {
      return { value, end: index + 1 }
    } else {
      value += char
    }
  }

  return null
}

function skipWhitespace(source: string, start: number): number {
  let index = start
  while (/[\s,]/.test(source[index] ?? '')) {
    index += 1
  }
  return index
}

function skipWhitespaceAndEquals(source: string, start: number): number {
  let index = skipWhitespace(source, start)
  if (source[index] === '=') {
    index += 1
  }
  return skipWhitespace(source, index)
}

function indent(value: string, spaces: number): string {
  const prefix = ' '.repeat(spaces)
  return value
    .split('\n')
    .map((line) => (line ? `${prefix}${line}` : line))
    .join('\n')
}
