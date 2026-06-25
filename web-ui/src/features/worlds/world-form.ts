import type { LevelPayload } from '@/features/levels/level.api'

export interface WorldForm {
  levelName: string
  uuid: string
  is_master: boolean
  server_ini: string
  leveldataoverride: string
  modoverrides: string
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
    server_ini: form.server_ini,
    leveldataoverride: form.leveldataoverride,
    modoverrides: form.modoverrides,
  }
}
