import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import * as routerModule from '@/app/router'
import * as authApi from '@/features/auth/auth.api'

const { createAppRouter } = routerModule

vi.mock('@/features/auth/auth.api', () => ({
  getUser: vi.fn(),
  login: vi.fn(),
  logout: vi.fn(),
}))

const getUser = vi.mocked(authApi.getUser)

describe('router guard', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    getUser.mockRejectedValue(new Error('未登录'))
  })

  it('redirects protected routes to login when no user is loaded', async () => {
    const router = createAppRouter()

    await router.push('/panel')
    await router.isReady()

    expect(router.currentRoute.value.path).toBe('/login')
    expect(router.currentRoute.value.query.redirect).toBe('/panel')
  })

  it('does not export an unused singleton router', () => {
    expect('router' in routerModule).toBe(false)
  })

  it('does not fetch the current user for public auth routes', async () => {
    const router = createAppRouter()

    await router.push('/login')
    await router.isReady()

    expect(router.currentRoute.value.path).toBe('/login')
    expect(getUser).not.toHaveBeenCalled()
  })
})
