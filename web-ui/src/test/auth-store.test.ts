import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import * as authApi from '@/features/auth/auth.api'
import { useAuthStore } from '@/shared/stores/auth'
import type { ApiEnvelope } from '@/shared/api/types'
import type { UserProfile } from '@/shared/types/domain'

vi.mock('@/features/auth/auth.api', () => ({
  getUser: vi.fn(),
  login: vi.fn(),
  logout: vi.fn(),
}))

const getUser = vi.mocked(authApi.getUser)
const login = vi.mocked(authApi.login)
const logout = vi.mocked(authApi.logout)

function success<T>(data: T): ApiEnvelope<T> {
  return {
    code: 0,
    data,
  }
}

describe('auth store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('fetches the current user and marks auth as initialized', async () => {
    const profile: UserProfile = {
      username: 'admin',
      displayName: '管理员',
    }
    getUser.mockResolvedValue(success(profile))

    const store = useAuthStore()

    await expect(store.fetchCurrentUser()).resolves.toEqual(profile)

    expect(getUser).toHaveBeenCalledOnce()
    expect(store.user?.displayName || store.user?.username).toBe('管理员')
    expect(store.initialized).toBe(true)
    expect(store.loading).toBe(false)
    expect(store.isAuthenticated).toBe(true)
  })

  it('logs in with username and password and stores the returned profile', async () => {
    const profile: UserProfile = {
      username: 'admin',
    }
    login.mockResolvedValue(success(profile))

    const store = useAuthStore()

    await expect(store.loginWithPassword('admin', 'secret')).resolves.toEqual(profile)

    expect(login).toHaveBeenCalledWith({
      username: 'admin',
      password: 'secret',
    })
    expect(store.user?.username).toBe('admin')
    expect(store.loading).toBe(false)
    expect(store.isAuthenticated).toBe(true)
  })

  it('uses the Chinese fallback message when login is rejected without a backend message', async () => {
    login.mockResolvedValue({
      code: 401,
      data: null as unknown as UserProfile,
    })

    const store = useAuthStore()

    await expect(store.loginWithPassword('admin', 'wrong')).rejects.toThrow('登录失败')

    expect(store.user).toBeNull()
    expect(store.loading).toBe(false)
    expect(store.isAuthenticated).toBe(false)
  })

  it('logs out through the API and clears local auth state', async () => {
    logout.mockResolvedValue(success(null))

    const store = useAuthStore()
    store.user = {
      username: 'admin',
    }

    await store.logoutUser()

    expect(logout).toHaveBeenCalledOnce()
    expect(store.user).toBeNull()
    expect(store.initialized).toBe(true)
    expect(store.isAuthenticated).toBe(false)
  })
})
