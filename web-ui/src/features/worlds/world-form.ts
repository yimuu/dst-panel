import type { LevelPayload } from '@/features/levels/level.api'
import type { ServerIniPayload } from '@/shared/types/domain'

export interface WorldForm {
  levelName: string
  uuid: string
  is_master: boolean
  server_ini: string
  leveldataoverride: string
  modoverrides: string
}

const masterServerIniDefault: ServerIniPayload = {
  server_port: 10999,
  is_master: true,
  name: 'Master',
  id: 10000,
  encode_user_path: true,
  authentication_port: 8766,
  master_server_port: 27016,
}

const cavesServerIniDefault: ServerIniPayload = {
  server_port: 10998,
  is_master: false,
  name: 'Caves',
  id: 10010,
  encode_user_path: true,
  authentication_port: 8766,
  master_server_port: 27016,
}

export function createEmptyWorldForm(): WorldForm {
  return {
    levelName: 'Master',
    uuid: 'Master',
    is_master: true,
    server_ini: '',
    leveldataoverride: '',
    modoverrides: '',
  }
}

export function normalizeWorldForm(form: WorldForm): LevelPayload {
  return {
    levelName: form.levelName.trim(),
    uuid: form.uuid.trim(),
    is_master: form.is_master,
    server_ini: normalizeServerIni(form),
    leveldataoverride: form.leveldataoverride,
    modoverrides: form.modoverrides,
  }
}

function normalizeServerIni(form: WorldForm): ServerIniPayload {
  const text = form.server_ini.trim()
  const defaults = createDefaultServerIni(form)

  if (!text) {
    return defaults
  }

  if (text.startsWith('{')) {
    return mergeServerIni(defaults, parseServerIniJson(text))
  }

  return mergeServerIni(defaults, parseServerIniText(text))
}

function createDefaultServerIni(form: WorldForm): ServerIniPayload {
  const baseDefault = form.is_master ? masterServerIniDefault : cavesServerIniDefault
  const name = form.uuid.trim() || form.levelName.trim() || baseDefault.name

  return {
    ...baseDefault,
    is_master: form.is_master,
    name,
  }
}

function parseServerIniJson(text: string): Record<string, unknown> {
  try {
    const parsed = JSON.parse(text) as unknown

    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>
    }
  } catch {
    throw new Error('server.ini 格式无效')
  }

  throw new Error('server.ini 格式无效')
}

function parseServerIniText(text: string): Record<string, unknown> {
  const values: Record<string, string> = {}

  for (const line of text.split(/\r?\n/)) {
    const trimmedLine = line.trim()

    if (!trimmedLine || trimmedLine.startsWith('#') || trimmedLine.startsWith(';')) {
      continue
    }

    if (trimmedLine.startsWith('[') && trimmedLine.endsWith(']')) {
      continue
    }

    const separatorIndex = trimmedLine.indexOf('=')

    if (separatorIndex === -1) {
      continue
    }

    const key = trimmedLine.slice(0, separatorIndex).trim()
    const value = trimmedLine.slice(separatorIndex + 1).trim()

    if (key) {
      values[key] = value
    }
  }

  return values
}

function mergeServerIni(
  defaults: ServerIniPayload,
  values: Record<string, unknown>,
): ServerIniPayload {
  return {
    server_port: toNumber(values.server_port, defaults.server_port),
    is_master: toBoolean(values.is_master, defaults.is_master),
    name: toStringValue(values.name, defaults.name),
    id: toNumber(values.id, defaults.id),
    encode_user_path: toBoolean(values.encode_user_path, defaults.encode_user_path),
    authentication_port: toNumber(values.authentication_port, defaults.authentication_port),
    master_server_port: toNumber(values.master_server_port, defaults.master_server_port),
  }
}

function toNumber(value: unknown, fallback: number): number {
  if (typeof value === 'number' && Number.isSafeInteger(value) && value >= 0) {
    return value
  }

  if (typeof value === 'string') {
    const trimmedValue = value.trim()

    if (/^\d+$/.test(trimmedValue)) {
      const parsed = Number(trimmedValue)

      if (Number.isSafeInteger(parsed)) {
        return parsed
      }
    }
  }

  return fallback
}

function toBoolean(value: unknown, fallback: boolean): boolean {
  if (typeof value === 'boolean') {
    return value
  }

  if (typeof value === 'string') {
    const normalized = value.trim().toLowerCase()

    if (normalized === 'true') {
      return true
    }

    if (normalized === 'false') {
      return false
    }
  }

  return fallback
}

function toStringValue(value: unknown, fallback: string): string {
  return typeof value === 'string' && value.trim() ? value.trim() : fallback
}
