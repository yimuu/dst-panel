import {
  CloudServerOutlined,
  DashboardOutlined,
  FileProtectOutlined,
  GithubOutlined,
  HomeOutlined,
  ProfileOutlined,
  ReadOutlined,
  SettingOutlined,
  TeamOutlined,
  ToolOutlined,
} from '@ant-design/icons'
import type { MenuDataItem } from '@ant-design/pro-components'

import { routes } from '@/shared/config/routes'

export interface AdminMenuItem extends MenuDataItem {
  path: string
  defaultPath?: string
  routePath?: string
  name: string
  children?: AdminMenuItem[]
}

export const adminMenuItems: AdminMenuItem[] = [
  { path: routes.dashboard, name: '统计面板', icon: <DashboardOutlined /> },
  { path: routes.panel, name: '面板操作', icon: <CloudServerOutlined /> },
  {
    path: '/home-group',
    defaultPath: routes.clusterIni,
    name: '房间设置',
    icon: <HomeOutlined />,
    children: [
      { path: '/room-settings', routePath: routes.clusterIni, name: '房间设置' },
      { path: routes.adminlist, name: '管理员列表' },
      { path: routes.whitelist, name: '白名单列表' },
      { path: routes.blacklist, name: '黑名单列表' },
    ],
  },
  {
    path: '/levels-group',
    defaultPath: routes.levels,
    name: '世界设置',
    icon: <ToolOutlined />,
    children: [
      { path: '/levels-settings', routePath: routes.levels, name: '世界设置' },
      { path: routes.selectorMod, name: '多层选择器' },
      { path: routes.preinstall, name: '世界模板' },
      { path: routes.genMap, name: '预览地图' },
    ],
  },
  { path: routes.mod, name: '模组设置', icon: <ProfileOutlined /> },
  { path: routes.backup, name: '存档备份', icon: <FileProtectOutlined /> },
  { path: routes.playerLog, name: '玩家日志', icon: <TeamOutlined /> },
  { path: routes.setting, name: '系统设置', icon: <SettingOutlined /> },
  { path: routes.lobby, name: '大厅列表', icon: <CloudServerOutlined /> },
  { path: routes.help, name: '帮助文档', icon: <ReadOutlined /> },
  {
    path: 'https://github.com/carrot-hu23/dst-admin-go',
    name: '源码仓库',
    icon: <GithubOutlined />,
  },
]

export function flattenAdminMenuItems(items: AdminMenuItem[] = adminMenuItems): AdminMenuItem[] {
  return items.flatMap((item) =>
    item.children ? [item, ...flattenAdminMenuItems(item.children)] : [item],
  )
}

export function getSelectedMenuKeys(
  pathname: string,
  items: AdminMenuItem[] = adminMenuItems,
): string[] {
  return flattenAdminMenuItems(items)
    .filter((item) => (item.routePath ?? item.path) === pathname)
    .map((item) => item.path)
}

export function getOpenMenuKeys(pathname: string): string[] {
  if (pathname.startsWith('/home/')) {
    return ['/home-group']
  }

  if (pathname.startsWith('/levels/')) {
    return ['/levels-group']
  }

  return []
}

export function getMenuNavigationPath(
  item: Pick<AdminMenuItem, 'children' | 'defaultPath' | 'path' | 'routePath'>,
): string {
  if (item.routePath) {
    return item.routePath
  }

  if (item.children?.length && item.defaultPath) {
    return item.defaultPath
  }

  return item.path
}

export function getGroupedMenuNavigationPath(
  name: string,
  items: AdminMenuItem[] = adminMenuItems,
): string | undefined {
  const matched = items.find((item) => item.children && item.name === name)
  return matched ? getMenuNavigationPath(matched) : undefined
}
