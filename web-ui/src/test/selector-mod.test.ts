import { describe, expect, it } from 'vitest'

import {
  DEFAULT_SELECTOR_WORKSHOP_ID,
  applySelectorWorldConfigToModoverrides,
  normalizeSelectorWorkshopId,
  parseSelectorWorldConfig,
  worldConfigFromLevels,
} from '@/features/levels/selector-mod'
import type { WorldLevel } from '@/features/levels/level.api'

describe('selector mod model', () => {
  it('normalizes workshop ids to the selector mod folder format', () => {
    expect(normalizeSelectorWorkshopId('1754389029')).toBe(DEFAULT_SELECTOR_WORKSHOP_ID)
    expect(normalizeSelectorWorkshopId('workshop-1754389029')).toBe(DEFAULT_SELECTOR_WORKSHOP_ID)
  })

  it('parses selector world_config entries from modoverrides.lua', () => {
    const parsed = parseSelectorWorldConfig(
      `return {
        ["workshop-1754389029"] = {
          configuration_options = {
            world_config = {
              ["forest"] = {
                name = "森林",
                category = "地上",
                galleryful = 6,
                extra = true,
                is_cave = false,
                invisible = false,
                note = "主世界",
              },
            },
          },
          enabled = true,
        },
      }`,
      DEFAULT_SELECTOR_WORKSHOP_ID,
    )

    expect(parsed).toEqual([
      expect.objectContaining({
        id: 'forest',
        name: '森林',
        category: '地上',
        galleryful: 6,
        extra: true,
        is_cave: false,
        invisible: false,
        note: '主世界',
      }),
    ])
  })

  it('writes selector config into empty modoverrides and preserves other mods', () => {
    const updated = applySelectorWorldConfigToModoverrides(
      `return { ["workshop-123"] = { enabled = true } }`,
      DEFAULT_SELECTOR_WORKSHOP_ID,
      [
        {
          id: 'forest',
          name: '森林',
          category: '地上',
          galleryful: 6,
          extra: false,
          is_cave: false,
          invisible: false,
          note: '主世界',
        },
      ],
    )

    expect(updated).toContain('["workshop-123"]')
    expect(updated).toContain('["workshop-1754389029"]')
    expect(updated).toContain('world_config')
    expect(updated).toContain('["forest"]')
    expect(updated).toContain('name = "森林"')
  })

  it('creates default selector rows from world levels', () => {
    const levels: WorldLevel[] = [
      worldLevel('森林', 'Master', true),
      worldLevel('洞穴', 'Caves', false),
    ]

    expect(worldConfigFromLevels(levels)).toEqual([
      expect.objectContaining({ id: 'Master', name: '森林', is_cave: false }),
      expect.objectContaining({ id: 'Caves', name: '洞穴', is_cave: true }),
    ])
  })
})

function worldLevel(levelName: string, uuid: string, isMaster: boolean): WorldLevel {
  return {
    levelName,
    uuid,
    is_master: isMaster,
    leveldataoverride: 'return {}',
    modoverrides: 'return {}',
    server_ini: {
      server_port: 11000,
      is_master: isMaster,
      name: uuid,
      id: 1,
      encode_user_path: true,
      authentication_port: 8766,
      master_server_port: 27016,
    },
  }
}
