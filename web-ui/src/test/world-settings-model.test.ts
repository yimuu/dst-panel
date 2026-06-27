import { describe, expect, it } from 'vitest'

import {
  buildWorldSettingGroups,
  normalizeWorldOptionValue,
} from '@/features/worlds/world-settings-model'

describe('world settings model', () => {
  it('keeps known values and falls back to default display values', () => {
    expect(normalizeWorldOptionValue('often')).toBe('often')
    expect(normalizeWorldOptionValue(undefined)).toBe('default')
    expect(normalizeWorldOptionValue('')).toBe('default')
  })

  it('sorts groups by order and exposes item option labels', () => {
    const groups = buildWorldSettingGroups({
      resources: {
        order: 2,
        text: '资源',
        atlas: { name: 'worldgen_customization', width: 2048, height: 1024, item_size: 128 },
        desc: { default: '默认', often: '较多' },
        items: {
          grass: { text: '草', value: 'often', image: { x: 0.25, y: 0.125 } },
        },
      },
      global: {
        order: 1,
        text: '全局',
        atlas: { name: 'worldsettings_customization', width: 2048, height: 1024, item_size: 128 },
        desc: { default: '默认' },
        items: {
          autumn: { text: '秋', value: undefined, image: { x: 0, y: 0 } },
        },
      },
    })

    expect(groups.map((group) => group.title)).toEqual(['全局', '资源'])
    expect(groups[0]?.items[0]).toMatchObject({
      key: 'autumn',
      label: '秋',
      value: 'default',
      atlasName: 'worldsettings_customization',
      options: [{ label: '默认', value: 'default' }],
    })
    expect(groups[1]?.items[0]?.options).toEqual([
      { label: '默认', value: 'default' },
      { label: '较多', value: 'often' },
    ])
  })

  it('falls back to the default option when a real json group has null desc', () => {
    const groups = buildWorldSettingGroups({
      misc: {
        order: 1,
        text: '其他',
        atlas: { name: 'worldsettings_customization', width: 2048, height: 1024, item_size: 128 },
        desc: null,
        items: {
          specialevent: { text: '活动', value: 'none', image: { x: 0, y: 0 } },
        },
      },
    })

    expect(groups[0]?.items[0]?.options).toEqual([{ label: '默认', value: 'default' }])
    expect(groups[0]?.items[0]?.value).toBe('default')
  })
})
