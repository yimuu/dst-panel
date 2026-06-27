import { describe, expect, it } from 'vitest'

import {
  formatWorkshopId,
  getModDisplayName,
  isModEnabled,
  type ModSummary,
} from '@/features/mods/mod-model'

describe('mod model', () => {
  it('formats workshop ids and enabled state', () => {
    expect(formatWorkshopId('workshop-378160973')).toBe('378160973')
    expect(formatWorkshopId('378160973')).toBe('378160973')
    expect(formatWorkshopId(' workshop-378160973 ')).toBe('378160973')
    expect(isModEnabled({ enabled: true })).toBe(true)
    expect(isModEnabled({ enabled: false })).toBe(false)
  })

  it('uses the mod name first and falls back to workshop id', () => {
    expect(getModDisplayName({ name: 'Global Positions', modid: '378160973' })).toBe(
      'Global Positions',
    )
    expect(getModDisplayName({ modid: 'workshop-378160973' } as ModSummary)).toBe('378160973')
  })
})
