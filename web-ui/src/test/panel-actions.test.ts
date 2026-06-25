import { describe, expect, it } from 'vitest'

import { getPanelActionLabel, isLevelActionDisabled } from '@/features/panel/panel-actions'
import type { LevelSummary } from '@/shared/types/domain'

describe('panel actions', () => {
  const runningLevel: LevelSummary = {
    uuid: '1',
    levelName: 'Master',
    is_master: true,
    status: true,
  }
  const stoppedLevel: LevelSummary = {
    uuid: '2',
    levelName: 'Caves',
    is_master: false,
    status: false,
  }

  it('labels level operations in Chinese', () => {
    expect(getPanelActionLabel('start')).toBe('启动')
    expect(getPanelActionLabel('stop')).toBe('停止')
    expect(getPanelActionLabel('restart')).toBe('重启')
  })

  it('disables impossible start and stop actions based on runtime state', () => {
    expect(isLevelActionDisabled(runningLevel, 'start')).toBe(true)
    expect(isLevelActionDisabled(runningLevel, 'stop')).toBe(false)
    expect(isLevelActionDisabled(stoppedLevel, 'start')).toBe(false)
    expect(isLevelActionDisabled(stoppedLevel, 'stop')).toBe(true)
  })
})
