import { describe, expect, it } from 'vitest'

import { adminMenuItems } from '@/layouts/menu'
import { routes } from '@/shared/config/routes'

describe('admin menu', () => {
  it('contains the core admin route paths', () => {
    const paths = JSON.stringify(adminMenuItems)

    expect(paths).toContain('/panel')
    expect(paths).toContain(routes.clusterIni)
    expect(paths).toContain('/levels/levels')
    expect(paths).toContain('/mod')
    expect(paths).toContain('/backup')
    expect(paths).toContain('/setting')
  })

  it('shows the completed room submenu entries', () => {
    const roomMenu = adminMenuItems.find((item) => item.label === '房间')

    expect(roomMenu?.children?.map((item) => item.path)).toEqual([routes.clusterIni])
    expect(roomMenu?.children?.map((item) => item.label)).toEqual(['集群设置'])
  })

  it('hides unfinished routes from the navigation menu', () => {
    const paths = JSON.stringify(adminMenuItems)

    expect(paths).not.toContain(routes.adminlist)
    expect(paths).not.toContain(routes.whitelist)
    expect(paths).not.toContain(routes.blacklist)
    expect(paths).not.toContain(routes.selectorMod)
    expect(paths).not.toContain(routes.preinstall)
    expect(paths).not.toContain(routes.genMap)
  })
})
