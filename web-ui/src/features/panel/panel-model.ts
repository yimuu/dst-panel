import type { LevelSummary } from '@/shared/types/domain'

export type PanelAction = 'start' | 'stop' | 'restart'

export function getPanelActionLabel(action: PanelAction): string {
  return {
    start: '启动世界',
    stop: '停止世界',
    restart: '重启世界',
  }[action]
}

export function getLevelActionTarget(level: LevelSummary): string {
  return level.levelName || level.name || ''
}
