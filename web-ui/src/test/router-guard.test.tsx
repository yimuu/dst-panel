import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import type { AxiosResponse } from 'axios'
import { afterEach, describe, expect, it, vi } from 'vitest'

import App from '@/app/App'
import { getAuthRedirect, readAuthRouteState, setAuthRouteState } from '@/features/auth/auth-state'
import { api } from '@/shared/api/http'
import { routes } from '@/shared/config/routes'

const originalAdapter = api.defaults.adapter

function mockApiResponse(data: unknown): AxiosResponse {
  return {
    data,
    status: 200,
    statusText: 'OK',
    headers: {},
    config: {},
  } as AxiosResponse
}

afterEach(() => {
  api.defaults.adapter = originalAdapter
  window.sessionStorage.clear()
  window.location.hash = ''
})

describe('auth route decisions', () => {
  it('sends first-run users to init', () => {
    expect(
      getAuthRedirect({
        firstRun: true,
        authenticated: false,
        publicRoute: false,
        path: routes.panel,
      }),
    ).toBe(routes.init)
  })

  it('sends anonymous protected users to login', () => {
    expect(
      getAuthRedirect({
        firstRun: false,
        authenticated: false,
        publicRoute: false,
        path: routes.panel,
      }),
    ).toBe(routes.login)
  })

  it('allows authenticated protected routes', () => {
    expect(
      getAuthRedirect({
        firstRun: false,
        authenticated: true,
        publicRoute: false,
        path: routes.panel,
      }),
    ).toBeUndefined()
  })

  it('redirects anonymous users away from protected routes', async () => {
    window.location.hash = routes.panel
    window.sessionStorage.clear()
    api.defaults.adapter = async (config) => {
      if (config.url === '/api/init') {
        return mockApiResponse({ code: 400, msg: 'is not first', data: null })
      }

      return mockApiResponse({ code: 401, msg: '未登录', data: null })
    }

    render(<App />)

    expect(await screen.findByRole('button', { name: /登\s*录/ })).toBeInTheDocument()
    expect(screen.queryByText('服务器信息')).not.toBeInTheDocument()
  })

  it('rejects stale local sessions when the backend user check fails', async () => {
    window.location.hash = routes.panel
    setAuthRouteState({ firstRun: false, authenticated: true })
    const adapter = vi.fn(async (config) => {
      if (config.url === '/api/init') {
        return mockApiResponse({ code: 400, msg: 'is not first', data: null })
      }

      return mockApiResponse({ code: 401, msg: '未登录', data: null })
    })
    api.defaults.adapter = adapter

    render(<App />)

    await waitFor(() => {
      expect(adapter).toHaveBeenCalled()
    })
    expect(await screen.findByRole('button', { name: /登\s*录/ })).toBeInTheDocument()
    expect(screen.queryByText('服务器信息')).not.toBeInTheDocument()
  })

  it('allows protected routes for valid backend sessions without local state', async () => {
    window.location.hash = routes.panel
    const adapter = vi.fn(async (config) => {
      if (config.url === '/api/init') {
        return mockApiResponse({ code: 400, msg: 'is not first', data: null })
      }

      return mockApiResponse({ code: 200, msg: 'Init user success', data: { username: 'admin' } })
    })
    api.defaults.adapter = adapter

    render(<App />)

    expect(await screen.findByText('服务器信息')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /登\s*录/ })).not.toBeInTheDocument()
  })

  it('sends protected routes to init when the backend is in first-run mode', async () => {
    window.location.hash = routes.panel
    const adapter = vi.fn(async () => mockApiResponse({ code: 200, msg: 'is first', data: null }))
    api.defaults.adapter = adapter

    render(<App />)

    expect(await screen.findByText('初始化管理员')).toBeInTheDocument()
    expect(screen.queryByText('服务器信息')).not.toBeInTheDocument()
  })

  it('does not unlock protected routes when login fails', async () => {
    window.location.hash = routes.login
    const adapter = vi.fn(async (config) => {
      if (config.url === '/api/init') {
        return mockApiResponse({ code: 400, msg: 'is not first', data: null })
      }

      return mockApiResponse({ code: 401, msg: 'User authentication failed', data: null })
    })
    api.defaults.adapter = adapter

    render(<App />)

    fireEvent.change(await screen.findByPlaceholderText('请输入用户名'), {
      target: { value: 'wrong' },
    })
    fireEvent.change(screen.getByPlaceholderText('请输入密码'), { target: { value: 'bad' } })
    fireEvent.click(screen.getByRole('button', { name: /登\s*录/ }))

    await waitFor(() => {
      expect(adapter).toHaveBeenCalledWith(expect.objectContaining({ url: '/api/login' }))
    })
    await waitFor(() => {
      expect(readAuthRouteState().authenticated).toBe(false)
    })
    expect(screen.queryByText('服务器信息')).not.toBeInTheDocument()
  })
})
