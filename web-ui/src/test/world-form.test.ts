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

  it('trims submitted world names and converts ini text into backend server.ini shape', () => {
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
      server_ini: {
        server_port: 11001,
        is_master: false,
        name: 'Caves',
        id: 10010,
        encode_user_path: true,
        authentication_port: 8766,
        master_server_port: 27016,
      },
    })
  })

  it('uses a backend-shaped default server.ini object when text is empty', () => {
    expect(
      normalizeWorldForm({
        ...createEmptyWorldForm(),
        server_ini: '',
      }),
    ).toMatchObject({
      server_ini: {
        server_port: 10999,
        is_master: true,
        name: 'Master',
        id: 10000,
        encode_user_path: true,
        authentication_port: 8766,
        master_server_port: 27016,
      },
    })
  })

  it('parses JSON server.ini text and merges it with defaults', () => {
    expect(
      normalizeWorldForm({
        levelName: '森林',
        uuid: 'Master',
        is_master: true,
        server_ini: '{ "server_port": 12000, "id": 42, "encode_user_path": false }',
        leveldataoverride: '',
        modoverrides: '',
      }),
    ).toMatchObject({
      server_ini: {
        server_port: 12000,
        is_master: true,
        name: 'Master',
        id: 42,
        encode_user_path: false,
        authentication_port: 8766,
        master_server_port: 27016,
      },
    })
  })

  it('throws a Chinese error for invalid JSON server.ini text', () => {
    expect(() =>
      normalizeWorldForm({
        ...createEmptyWorldForm(),
        server_ini: '{ invalid json',
      }),
    ).toThrow('server.ini 格式无效')
  })
})
