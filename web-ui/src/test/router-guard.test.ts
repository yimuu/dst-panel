import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { createAppRouter } from '@/app/router'
import * as authApi from '@/features/auth/auth.api'

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
})
