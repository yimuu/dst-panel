import { describe, expect, it } from 'vitest'

import { adminMenuItems, flattenAdminMenuItems } from '@/layouts/menu'
import { routes } from '@/shared/config/routes'

describe('admin menu', () => {
  it('contains official preview route groups', () => {
    const paths = flattenAdminMenuItems(adminMenuItems).map((item) => item.path)

    expect(paths).toContain(routes.dashboard)
    expect(paths).toContain(routes.panel)
    expect(paths).toContain(routes.clusterIni)
    expect(paths).toContain(routes.levels)
    expect(paths).toContain(routes.mod)
    expect(paths).toContain(routes.backup)
  })

  it('uses Chinese visible labels', () => {
    const labels = flattenAdminMenuItems(adminMenuItems).map((item) => item.name)

    expect(labels).toContain('统计面板')
    expect(labels).toContain('源码仓库')
    expect(labels).not.toContain('Dashboard')
    expect(labels).not.toContain('Github')
  })
})
