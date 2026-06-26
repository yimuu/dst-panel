import { describe, expect, it } from 'vitest'

import { normalizePanelSettings } from '@/features/settings/settings-form'

describe('settings form', () => {
  it('trims text fields and preserves boolean values', () => {
    expect(
      normalizePanelSettings({
        panelName: '  DST 管理面板  ',
        enableRegister: false,
        steamApiKey: '  key  ',
      }),
    ).toEqual({
      panelName: 'DST 管理面板',
      enableRegister: false,
      steamApiKey: 'key',
    })
  })
})
