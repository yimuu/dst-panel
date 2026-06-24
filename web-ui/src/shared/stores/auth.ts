import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import * as authApi from '@/features/auth/auth.api'
import { isApiSuccess } from '@/shared/api/http'
import type { LoginRequest, UserProfile } from '@/shared/types/domain'

const fallbackLoginError = '登录失败'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<UserProfile | null>(null)
  const loading = ref(false)
  const initialized = ref(false)

  const isAuthenticated = computed(() => user.value !== null)

  async function fetchCurrentUser(): Promise<UserProfile | null> {
    loading.value = true

    try {
      const response = await authApi.getUser()
      user.value = isApiSuccess(response) ? response.data : null
      return user.value
    } catch {
      user.value = null
      return null
    } finally {
      initialized.value = true
      loading.value = false
    }
  }

  async function loginWithPassword(payload: LoginRequest): Promise<UserProfile> {
    loading.value = true

    try {
      const response = await authApi.login(payload)

      if (!isApiSuccess(response)) {
        throw new Error(response.msg || response.message || fallbackLoginError)
      }

      user.value = response.data
      initialized.value = true
      return response.data
    } finally {
      loading.value = false
    }
  }

  async function logoutUser(): Promise<void> {
    loading.value = true

    try {
      await authApi.logout()
    } finally {
      clearAuth()
    }
  }

  function clearAuth(): void {
    user.value = null
    initialized.value = true
    loading.value = false
  }

  return {
    user,
    loading,
    initialized,
    isAuthenticated,
    fetchCurrentUser,
    loginWithPassword,
    logoutUser,
    clearAuth,
  }
})
