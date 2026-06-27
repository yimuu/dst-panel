import { ProLayout } from '@ant-design/pro-components'
import { Tag } from 'antd'
import { Outlet, useLocation, useNavigate } from 'react-router'

import { AppHeader } from './AppHeader'
import { adminMenuItems } from './menu'

export default function AdminLayout() {
  const navigate = useNavigate()
  const location = useLocation()

  return (
    <ProLayout
      title="Dst-admin-go"
      logo={false}
      layout="mix"
      fixedHeader
      fixSiderbar
      menuDataRender={() => adminMenuItems}
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
          <Tag color="processing">v1.6.1</Tag>
          <span className="sr-only">饥荒联机版管理面板</span>
        </div>
      )}
      actionsRender={() => [<AppHeader key="app-header" />]}
      menuItemRender={(item, dom) => {
        if (!item.path) {
          return dom
        }

        if (item.path.startsWith('http')) {
          return (
            <a href={item.path} target="_blank" rel="noreferrer">
              {dom}
            </a>
          )
        }

        return <button onClick={() => navigate(item.path ?? '/')}>{dom}</button>
      }}
    >
      <main className="admin-page">
        <Outlet />
      </main>
    </ProLayout>
  )
}
