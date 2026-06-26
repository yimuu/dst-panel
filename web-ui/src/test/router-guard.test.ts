import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import * as routerModule from '@/app/router'
import * as authApi from '@/features/auth/auth.api'

const { createAppRouter } = routerModule

vi.mock('@/features/auth/auth.api', () => ({
  getInitStatus: vi.fn(),
  getUser: vi.fn(),
  initialize: vi.fn(),
  login: vi.fn(),
  logout: vi.fn(),
}))

const getInitStatus = vi.mocked(authApi.getInitStatus)
const getUser = vi.mocked(authApi.getUser)

describe('router guard', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    getInitStatus.mockResolvedValue({ code: 400, msg: 'is not first', data: null })
    getUser.mockRejectedValue(new Error('未登录'))
  })

  it('redirects protected routes to login when no user is loaded', async () => {
    const router = createAppRouter()

    await router.push('/panel')
    await router.isReady()

    expect(router.currentRoute.value.path).toBe('/login')
    expect(router.currentRoute.value.query.redirect).toBe('/panel')
  })

  it('redirects protected routes to init during first-run setup', async () => {
    getInitStatus.mockResolvedValue({ code: 200, msg: 'is first', data: null })
    const router = createAppRouter()

    await router.push('/panel')
    await router.isReady()

    expect(router.currentRoute.value.path).toBe('/init')
    expect(router.currentRoute.value.query.redirect).toBe('/panel')
    expect(getUser).not.toHaveBeenCalled()
  })

  it('redirects login to init while first-run setup is required', async () => {
    getInitStatus.mockResolvedValue({ code: 200, msg: 'is first', data: null })
    const router = createAppRouter()

    await router.push('/login')
    await router.isReady()

    expect(router.currentRoute.value.path).toBe('/init')
  })

  it('redirects init to login after first-run setup has completed', async () => {
    const router = createAppRouter()

    await router.push('/init')
    await router.isReady()

    expect(router.currentRoute.value.path).toBe('/login')
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
    expect(getInitStatus).toHaveBeenCalled()
  })
})
