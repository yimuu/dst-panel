import { describe, expect, it } from 'vitest'

import {
  createModOverridesLua,
  formatWorkshopId,
  getModDisplayName,
  normalizeModConfig,
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

  it('creates modoverrides.lua from enabled mods and default options', () => {
    const lua = createModOverridesLua([
      {
        enabled: true,
        modid: '378160973',
        mod_config: [
          {
            name: 'difficulty',
            default: 'easy',
          },
          {
            name: 'client_only',
            default: false,
          },
        ],
      } as ModSummary,
      {
        enabled: false,
        modid: '345692228',
      } as ModSummary,
    ])

    expect(lua).toContain('["workshop-378160973"]')
    expect(lua).toContain('configuration_options')
    expect(lua).toContain('["difficulty"] = "easy"')
    expect(lua).toContain('["client_only"] = false')
    expect(lua).not.toContain('345692228')
  })

  it('normalizes backend mod_config objects with configuration_options', () => {
    const options = normalizeModConfig({
      name: 'Backend Mod',
      configuration_options: [
        {
          name: 'language',
          label: '语言',
          default: 'zh',
          options: [
            { description: '中文', data: 'zh' },
            { description: 'English', data: 'en' },
          ],
        },
      ],
    })

    expect(options).toHaveLength(1)
    expect(options[0]).toMatchObject({
      name: 'language',
      label: '语言',
      default: 'zh',
    })
  })
})
