import type { LevelSummary } from '@/shared/types/domain'

export type PanelAction = 'start' | 'stop' | 'restart'

export function getPanelActionLabel(action: PanelAction): string {
  return {
    start: '启动',
    stop: '停止',
    restart: '重启',
  }[action]
}

export function isLevelActionDisabled(level: LevelSummary, action: PanelAction): boolean {
  if (action === 'restart') {
    return false
  }

  if (action === 'start') {
    return Boolean(level.status)
  }

  return !level.status
}
