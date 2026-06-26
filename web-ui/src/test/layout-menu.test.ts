import { describe, expect, it } from 'vitest'

import { adminMenuItems } from '@/layouts/menu'
import { routes } from '@/shared/config/routes'

describe('admin menu', () => {
  it('contains the core admin route paths', () => {
    const paths = JSON.stringify(adminMenuItems)

    expect(paths).toContain('/panel')
    expect(paths).toContain(routes.clusterIni)
    expect(paths).toContain(routes.adminlist)
    expect(paths).toContain(routes.whitelist)
    expect(paths).toContain(routes.blacklist)
    expect(paths).toContain('/levels/levels')
    expect(paths).toContain(routes.selectorMod)
    expect(paths).toContain(routes.preinstall)
    expect(paths).toContain(routes.genMap)
    expect(paths).toContain('/mod')
    expect(paths).toContain('/backup')
    expect(paths).toContain('/setting')
  })

  it('shows the completed room submenu entries', () => {
    const roomMenu = adminMenuItems.find((item) => item.label === '房间')

    expect(roomMenu?.children?.map((item) => item.path)).toEqual([
      routes.clusterIni,
      routes.adminlist,
      routes.whitelist,
      routes.blacklist,
    ])
    expect(roomMenu?.children?.map((item) => item.label)).toEqual([
      '集群设置',
      '管理员列表',
      '白名单',
      '黑名单',
    ])
  })

  it('shows the completed world submenu entries', () => {
    const worldMenu = adminMenuItems.find((item) => item.label === '世界')

    expect(worldMenu?.children?.map((item) => item.path)).toEqual([
      routes.levels,
      routes.selectorMod,
      routes.preinstall,
      routes.genMap,
    ])
    expect(worldMenu?.children?.map((item) => item.label)).toEqual([
      '世界列表',
      '选择模组',
      '预设模板',
      '地图预览',
    ])
  })

  it('has no hidden routes left from the rebuilt room and world workflows', () => {
    const paths = JSON.stringify(adminMenuItems)

    expect(paths).toContain(routes.clusterIni)
    expect(paths).toContain(routes.adminlist)
    expect(paths).toContain(routes.whitelist)
    expect(paths).toContain(routes.blacklist)
    expect(paths).toContain(routes.selectorMod)
    expect(paths).toContain(routes.preinstall)
    expect(paths).toContain(routes.genMap)
  })
})
