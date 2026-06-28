import { ProLayout } from '@ant-design/pro-components'
import { Tag } from 'antd'
import { useMemo, useState } from 'react'
import { Outlet, useLocation, useNavigate } from 'react-router'

import { AppHeader } from './AppHeader'
import {
  adminMenuItems,
  getMenuNavigationPath,
  getOpenMenuKeys,
  getSelectedMenuKeys,
  type AdminMenuItem,
} from './menu'

export default function AdminLayout() {
  const navigate = useNavigate()
  const location = useLocation()
  const selectedMenuKeys = getSelectedMenuKeys(location.pathname)
  const routeOpenMenuKeys = useMemo(() => getOpenMenuKeys(location.pathname), [location.pathname])
  const [manualOpenMenuKeys, setManualOpenMenuKeys] = useState<string[]>([])
  const openMenuKeys = useMemo(
    () => mergeMenuKeys([...routeOpenMenuKeys, ...manualOpenMenuKeys]),
    [manualOpenMenuKeys, routeOpenMenuKeys],
  )
  const menuItems = useMemo(() => adminMenuItems, [])
  const routeClassName = `admin-route-${location.pathname
    .replace(/^\/+/, '')
    .replace(/[^a-zA-Z0-9]+/g, '-')}`

  return (
    <div>
      <ProLayout
        title="饥荒管理面板"
        className={routeClassName}
        logo={false}
        layout="mix"
        fixedHeader
        fixSiderbar
        menuDataRender={() => menuItems}
        openKeys={openMenuKeys}
        selectedKeys={selectedMenuKeys}
        menuProps={{
          openKeys: openMenuKeys,
          selectedKeys: selectedMenuKeys,
          onOpenChange: (keys) => setManualOpenMenuKeys(keys.map(String)),
        }}
        location={{ pathname: location.pathname }}
        token={{
          bgLayout: '#f0f2f5',
          header: {
            colorBgHeader: '#ffffff',
            colorTextMenu: '#1f1f1f',
          },
          sider: {
            colorMenuBackground: '#ffffff',
            colorTextMenu: '#1f1f1f',
            colorTextMenuSelected: '#4f46e5',
          },
        }}
        headerTitleRender={(_, title) => (
          <div className="app-brand">
            <span className="app-brand-title">{title}</span>
            <Tag color="processing">v1.0.0</Tag>
            <span className="sr-only">饥荒联机版管理面板</span>
          </div>
        )}
        actionsRender={() => [<AppHeader key="app-header" />]}
        menuItemRender={(item, dom) => {
          if (item.children) {
            return dom
          }

          if (!item.path) {
            return dom
          }

          const targetPath = getMenuNavigationPath(item as AdminMenuItem)
          if (item.path.startsWith('http')) {
            return (
              <a href={item.path} target="_blank" rel="noreferrer">
                {dom}
              </a>
            )
          }

          return (
            <button type="button" onClick={() => navigate(targetPath ?? '/')}>
              {dom}
            </button>
          )
        }}
      >
        <main className="admin-page">
          <Outlet />
        </main>
      </ProLayout>
    </div>
  )
}

function mergeMenuKeys(keys: string[]): string[] {
  return Array.from(new Set(keys))
}
