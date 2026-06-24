import { describe, expect, it } from 'vitest'

import { adminMenuItems } from '@/layouts/menu'

describe('admin menu', () => {
  it('contains the core admin route paths', () => {
    const paths = JSON.stringify(adminMenuItems)

    expect(paths).toContain('/panel')
    expect(paths).toContain('/levels/levels')
    expect(paths).toContain('/mod')
    expect(paths).toContain('/backup')
    expect(paths).toContain('/setting')
  })

  it('groups room and world routes under submenu entries', () => {
    const roomMenu = adminMenuItems.find((item) => item.path === '/home')
    const worldMenu = adminMenuItems.find((item) => item.path === '/levels')

    expect(roomMenu?.children?.map((item) => item.path)).toEqual([
      '/home/clusterIni',
      '/home/adminlist',
      '/home/whitelist',
      '/home/blacklist',
    ])
    expect(worldMenu?.children?.map((item) => item.path)).toEqual([
      '/levels/levels',
      '/levels/selectorMod',
      '/levels/preinstall',
      '/levels/genMap',
    ])
  })
})
