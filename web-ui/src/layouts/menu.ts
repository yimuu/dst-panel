import type { Component } from 'vue'
import {
  Box,
  DataBoard,
  Document,
  Files,
  FolderOpened,
  Help,
  HomeFilled,
  List,
  Lock,
  Monitor,
  Operation,
  Setting,
  Star,
  Tickets,
  User,
  VideoCamera,
} from '@element-plus/icons-vue'

import { routes } from '@/shared/config/routes'

export interface AdminMenuItem {
  path: string
  label: string
  icon?: Component
  children?: AdminMenuItem[]
}

export const adminMenuItems: AdminMenuItem[] = [
  {
    path: routes.dashboard,
    label: '仪表盘',
    icon: DataBoard,
  },
  {
    path: routes.panel,
    label: '面板',
    icon: Monitor,
  },
  {
    path: '/home',
    label: '房间',
    icon: HomeFilled,
    children: [
      {
        path: routes.clusterIni,
        label: '集群设置',
        icon: HomeFilled,
      },
      {
        path: routes.adminlist,
        label: '管理员列表',
        icon: User,
      },
      {
        path: routes.whitelist,
        label: '白名单',
        icon: Star,
      },
      {
        path: routes.blacklist,
        label: '黑名单',
        icon: Lock,
      },
    ],
  },
  {
    path: '/levels',
    label: '世界',
    icon: Operation,
    children: [
      {
        path: routes.levels,
        label: '世界',
        icon: List,
      },
      {
        path: routes.selectorMod,
        label: '选择模组',
        icon: Tickets,
      },
      {
        path: routes.preinstall,
        label: '预设模板',
        icon: FolderOpened,
      },
      {
        path: routes.genMap,
        label: '地图预览',
        icon: Document,
      },
    ],
  },
  {
    path: routes.mod,
    label: '模组',
    icon: Box,
  },
  {
    path: routes.backup,
    label: '备份',
    icon: Files,
  },
  {
    path: routes.playerLog,
    label: '玩家日志',
    icon: VideoCamera,
  },
  {
    path: routes.setting,
    label: '设置',
    icon: Setting,
  },
  {
    path: routes.lobby,
    label: '大厅',
    icon: HomeFilled,
  },
  {
    path: routes.help,
    label: '帮助',
    icon: Help,
  },
]

export function flattenAdminMenuItems(items: AdminMenuItem[] = adminMenuItems): AdminMenuItem[] {
  return items.flatMap((item) => (item.children ? flattenAdminMenuItems(item.children) : [item]))
}
