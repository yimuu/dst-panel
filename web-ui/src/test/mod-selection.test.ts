import { describe, expect, it } from 'vitest'

import { formatWorkshopId, toggleModId } from '@/features/mods/mod-selection'

describe('mod selection', () => {
  it('normalizes workshop ids', () => {
    expect(formatWorkshopId(' workshop-123456 ')).toBe('123456')
    expect(formatWorkshopId('123456')).toBe('123456')
  })

  it('toggles mod ids without duplicates', () => {
    expect(toggleModId(['1', '2'], '2')).toEqual(['1'])
    expect(toggleModId(['1'], '2')).toEqual(['1', '2'])
  })
})
