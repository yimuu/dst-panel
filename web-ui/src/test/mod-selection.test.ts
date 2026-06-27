import { describe, expect, it } from 'vitest'

import { toggleSelectedMod } from '@/features/mods/mod-selection'

describe('world mod selection', () => {
  it('adds and removes selected mod ids', () => {
    expect(toggleSelectedMod(['378160973'], '123')).toEqual(['378160973', '123'])
    expect(toggleSelectedMod(['378160973', '123'], '123')).toEqual(['378160973'])
  })

  it('ignores blank mod ids', () => {
    expect(toggleSelectedMod(['378160973'], '  ')).toEqual(['378160973'])
  })
})
