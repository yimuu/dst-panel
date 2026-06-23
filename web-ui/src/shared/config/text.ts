export const text = {
  app: {
    title: '饥荒管理控制台',
  },
  common: {
    loading: '加载中',
    empty: '暂无数据',
    error: '发生错误',
    save: '保存',
    refresh: '刷新',
    disabled: '已禁用',
  },
  auth: {
    login: '登录',
    username: '用户名',
    password: '密码',
    logout: '退出登录',
  },
  menu: {
    dashboard: '仪表盘',
    panel: '控制面板',
    home: '首页',
    clusterIni: '集群配置',
    adminlist: '管理员列表',
    whitelist: '白名单',
    blacklist: '黑名单',
    levels: '世界管理',
    selectorMod: '模组选择',
    preinstall: '预安装',
    genMap: '生成地图',
    mod: '模组管理',
    backup: '备份管理',
    playerLog: '玩家日志',
    setting: '系统设置',
    lobby: '大厅',
    help: '帮助',
    userProfile: '用户资料',
  },
} as const

export type AppText = typeof text
export type MenuTextKey = keyof AppText['menu']
