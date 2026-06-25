import { describe, expect, it } from 'vitest'

import { createEmptyWorldForm, normalizeWorldForm } from '@/features/worlds/world-form'

describe('world form', () => {
  it('creates a Chinese default master world form', () => {
    expect(createEmptyWorldForm()).toEqual({
      levelName: 'Master',
      uuid: 'Master',
      is_master: true,
      server_ini: '',
      leveldataoverride: '',
      modoverrides: '',
    })
  })

  it('trims submitted world names and keeps config strings intact', () => {
    expect(
      normalizeWorldForm({
        levelName: '  Caves  ',
        uuid: '  Caves  ',
        is_master: false,
        server_ini: '[NETWORK]\nserver_port = 11001',
        leveldataoverride: 'return {}',
        modoverrides: 'return {}',
      }),
    ).toMatchObject({
      levelName: 'Caves',
      uuid: 'Caves',
      is_master: false,
      server_ini: '[NETWORK]\nserver_port = 11001',
    })
  })
})
