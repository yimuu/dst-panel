import { describe, expect, it } from 'vitest'

import { getLevelActionTarget, getPanelActionLabel } from '@/features/panel/panel-model'

describe('panel model', () => {
  it('formats action labels and level targets', () => {
    expect(getPanelActionLabel('start')).toBe('启动世界')
    expect(getPanelActionLabel('stop')).toBe('停止世界')
    expect(getPanelActionLabel('restart')).toBe('重启世界')
    expect(getLevelActionTarget({ levelName: '森林' })).toBe('森林')
    expect(getLevelActionTarget({ name: '洞穴' })).toBe('洞穴')
  })
})
