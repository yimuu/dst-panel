import { describe, expect, it } from 'vitest'

import {
  adminMenuItems,
  flattenAdminMenuItems,
  getGroupedMenuNavigationPath,
  getMenuNavigationPath,
  getOpenMenuKeys,
  getSelectedMenuKeys,
} from '@/layouts/menu'
import { routes } from '@/shared/config/routes'

describe('admin menu', () => {
  it('contains official preview route groups', () => {
    const paths = flattenAdminMenuItems(adminMenuItems).map((item) => item.routePath ?? item.path)

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

  it('exposes room settings as the first child route in the room group', () => {
    const roomGroup = adminMenuItems.find((item) => item.name === '房间设置')

    expect(roomGroup?.path).toBe('/home-group')
    expect(roomGroup?.children?.[0]).toMatchObject({
      routePath: routes.clusterIni,
      name: '房间设置',
    })
    expect(roomGroup?.children?.[0]?.path).not.toBe(routes.clusterIni)
    expect(roomGroup?.children?.map((item) => item.path)).not.toContain(roomGroup?.path)
  })

  it('maps grouped menu titles to their default pages', () => {
    const roomGroup = adminMenuItems.find((item) => item.name === '房间设置')
    const worldGroup = adminMenuItems.find((item) => item.name === '世界设置')

    expect(roomGroup && getMenuNavigationPath(roomGroup)).toBe(routes.clusterIni)
    expect(worldGroup && getMenuNavigationPath(worldGroup)).toBe(routes.levels)
    expect(getGroupedMenuNavigationPath('房间设置')).toBe(routes.clusterIni)
    expect(getGroupedMenuNavigationPath('世界设置')).toBe(routes.levels)
  })

  it('does not use inherited group defaults for leaf menu navigation', () => {
    expect(
      getMenuNavigationPath({
        path: routes.adminlist,
        defaultPath: routes.clusterIni,
      }),
    ).toBe(routes.adminlist)
  })

  it('exposes world settings as the first child route in the world group', () => {
    const worldGroup = adminMenuItems.find((item) => item.name === '世界设置')

    expect(worldGroup?.path).toBe('/levels-group')
    expect(worldGroup?.children?.[0]).toMatchObject({
      routePath: routes.levels,
      name: '世界设置',
    })
    expect(worldGroup?.children?.[0]?.path).not.toBe(routes.levels)
    expect(worldGroup?.children?.map((item) => item.path)).not.toContain(worldGroup?.path)
    expect(routes.selectorMod.startsWith(worldGroup?.path ?? '')).toBe(false)
  })

  it('selects only the exact child route for world submenu pages', () => {
    const worldGroup = adminMenuItems.find((item) => item.name === '世界设置')
    const worldSettingsMenuKey = worldGroup?.children?.[0]?.path

    expect(getSelectedMenuKeys(routes.levels)).toEqual([worldSettingsMenuKey])
    expect(getSelectedMenuKeys(routes.selectorMod)).toEqual([routes.selectorMod])
  })

  it('selects only the exact child route for room submenu pages', () => {
    const roomGroup = adminMenuItems.find((item) => item.name === '房间设置')
    const roomSettingsMenuKey = roomGroup?.children?.[0]?.path

    expect(getSelectedMenuKeys(routes.clusterIni)).toEqual([roomSettingsMenuKey])
    expect(getSelectedMenuKeys(routes.adminlist)).toEqual([routes.adminlist])
  })

  it('opens the world submenu for every world route', () => {
    expect(getOpenMenuKeys(routes.levels)).toEqual(['/levels-group'])
    expect(getOpenMenuKeys(routes.selectorMod)).toEqual(['/levels-group'])
    expect(getOpenMenuKeys(routes.mod)).toEqual([])
  })

  it('opens the room submenu for every room route', () => {
    expect(getOpenMenuKeys(routes.clusterIni)).toEqual(['/home-group'])
    expect(getOpenMenuKeys(routes.adminlist)).toEqual(['/home-group'])
  })
})
