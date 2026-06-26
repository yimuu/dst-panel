import { describe, expect, it } from 'vitest'

import { adminMenuItems } from '@/layouts/menu'
import { routes } from '@/shared/config/routes'

describe('admin menu', () => {
  it('contains the core admin route paths', () => {
    const paths = JSON.stringify(adminMenuItems)

    expect(paths).toContain('/panel')
    expect(paths).toContain('/levels/levels')
    expect(paths).toContain('/mod')
    expect(paths).toContain('/backup')
    expect(paths).toContain('/setting')
  })

  it('hides unfinished routes from the navigation menu', () => {
    const paths = JSON.stringify(adminMenuItems)

    expect(paths).not.toContain(routes.clusterIni)
    expect(paths).not.toContain(routes.adminlist)
    expect(paths).not.toContain(routes.whitelist)
    expect(paths).not.toContain(routes.blacklist)
    expect(paths).not.toContain(routes.selectorMod)
    expect(paths).not.toContain(routes.preinstall)
    expect(paths).not.toContain(routes.genMap)
  })
})
