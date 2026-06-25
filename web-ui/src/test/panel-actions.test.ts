import { describe, expect, it } from 'vitest'

import {
  getLevelActionTarget,
  getPanelActionLabel,
  isLevelActionDisabled,
} from '@/features/panel/panel-actions'
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

  it('keeps restart enabled regardless of runtime state', () => {
    expect(isLevelActionDisabled(runningLevel, 'restart')).toBe(false)
    expect(isLevelActionDisabled(stoppedLevel, 'restart')).toBe(false)
  })

  it('prefers shard uuid over display level name for operations', () => {
    expect(getLevelActionTarget({ uuid: 'Master', levelName: '森林' })).toBe('Master')
  })

  it('falls back to trimmed level name when uuid is unavailable', () => {
    expect(getLevelActionTarget({ levelName: '  Caves  ' })).toBe('Caves')
  })

  it('returns an empty operation target for missing identifiers', () => {
    expect(getLevelActionTarget({})).toBe('')
    expect(getLevelActionTarget({ uuid: ' ', levelName: ' ' })).toBe('')
  })
})
