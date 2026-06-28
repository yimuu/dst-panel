import { fireEvent, render, screen, within } from '@testing-library/react'
import { App as AntApp, ConfigProvider } from 'antd'
import { MemoryRouter, Route, Routes } from 'react-router'
import { describe, expect, it } from 'vitest'

import AdminLayout from '@/layouts/AdminLayout'
import { routes } from '@/shared/config/routes'

function renderWithLayout(initialPath = routes.panel) {
  return render(
    <ConfigProvider>
      <AntApp>
        <MemoryRouter initialEntries={[initialPath]}>
          <Routes>
            <Route element={<AdminLayout />}>
              <Route path={routes.panel} element={<div>面板页面</div>} />
              <Route path={routes.clusterIni} element={<div>房间设置页面</div>} />
              <Route path={routes.adminlist} element={<div>管理员列表页面</div>} />
              <Route path={routes.levels} element={<div>世界设置页面</div>} />
              <Route path={routes.selectorMod} element={<div>多层选择器页面</div>} />
            </Route>
          </Routes>
        </MemoryRouter>
      </AntApp>
    </ConfigProvider>,
  )
}

async function clickSubmenuArrow(label: string) {
  const title = (await screen.findByText(label)).closest('.ant-menu-submenu-title')
  expect(title).not.toBeNull()
  const arrow = title?.querySelector('.ant-menu-submenu-arrow')
  expect(arrow).not.toBeNull()
  fireEvent.click(arrow as Element)
}

function clickMenuButton(container: HTMLElement, label: string) {
  const button = [...container.querySelectorAll('button')].find(
    (item) => item.textContent?.trim() === label,
  )
  expect(button).not.toBeUndefined()
  fireEvent.click(button as Element)
}

describe('admin layout navigation', () => {
  it('expands room and world route groups from the panel page', async () => {
    const { container } = renderWithLayout()

    expect(screen.getByText('面板页面')).toBeInTheDocument()
    await clickSubmenuArrow('房间设置')
    expect(screen.getByText('面板页面')).toBeInTheDocument()

    clickMenuButton(container, '房间设置')
    expect(await screen.findByText('房间设置页面')).toBeInTheDocument()

    expect(await within(container).findByText('管理员列表')).toBeInTheDocument()
    clickMenuButton(container, '管理员列表')
    expect(await screen.findByText('管理员列表页面')).toBeInTheDocument()

    await clickSubmenuArrow('世界设置')
    clickMenuButton(container, '世界设置')
    expect(await screen.findByText('世界设置页面')).toBeInTheDocument()

    expect(await within(container).findByText('多层选择器')).toBeInTheDocument()
    clickMenuButton(container, '多层选择器')
    expect(await screen.findByText('多层选择器页面')).toBeInTheDocument()
  })
})
