import type { Component } from 'vue'
import {
  DataBoard,
  Document,
  Files,
  FolderOpened,
  Help,
  HomeFilled,
  List,
  Lock,
  Operation,
  Setting,
  Star,
  Tickets,
  Tools,
  User,
  VideoCamera,
} from '@element-plus/icons-vue'

import { routes } from '@/shared/config/routes'

export interface AdminMenuItem {
  path: string
  label: string
  icon: Component
}

export const adminMenuItems: AdminMenuItem[] = [
  {
    path: routes.dashboard,
    label: '仪表盘',
    icon: DataBoard,
  },
  {
    path: routes.panel,
    label: '控制面板',
    icon: Operation,
  },
  {
    path: routes.clusterIni,
    label: '集群配置',
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
  {
    path: routes.levels,
    label: '世界管理',
    icon: List,
  },
  {
    path: routes.selectorMod,
    label: '模组选择',
    icon: Tickets,
  },
  {
    path: routes.preinstall,
    label: '预安装',
    icon: FolderOpened,
  },
  {
    path: routes.genMap,
    label: '生成地图',
    icon: Document,
  },
  {
    path: routes.mod,
    label: '模组管理',
    icon: Tools,
  },
  {
    path: routes.backup,
    label: '备份管理',
    icon: Files,
  },
  {
    path: routes.playerLog,
    label: '玩家日志',
    icon: VideoCamera,
  },
  {
    path: routes.setting,
    label: '系统设置',
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
  {
    path: routes.userProfile,
    label: '用户资料',
    icon: User,
  },
]
