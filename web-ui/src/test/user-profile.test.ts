import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import ElementPlus, { ElMessage } from 'element-plus'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import * as authApi from '@/features/auth/auth.api'
import UserProfilePage from '@/pages/UserProfilePage.vue'
import {
  getProfileAccountId,
  getProfileCreatedAt,
  getProfileDisplayName,
  getProfileRole,
  normalizeNewPassword,
  validateNewPassword,
} from '@/features/auth/user-profile'
import { useAuthStore } from '@/shared/stores/auth'

vi.mock('element-plus', async () => {
  const actual = await vi.importActual<typeof import('element-plus')>('element-plus')

  return {
    ...actual,
    ElMessage: {
      success: vi.fn(),
      error: vi.fn(),
    },
  }
})

vi.mock('@/features/auth/auth.api', () => ({
  changePassword: vi.fn(),
}))

const changePassword = vi.mocked(authApi.changePassword)
let wrapper: VueWrapper | undefined

function mountUserProfilePage(): VueWrapper {
  setActivePinia(createPinia())
  wrapper = mount(UserProfilePage, {
    global: {
      plugins: [ElementPlus],
    },
  })
  return wrapper
}

beforeEach(() => {
  vi.clearAllMocks()
  changePassword.mockResolvedValue({
    code: 0,
    data: null,
  })
})

afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
})

describe('user profile helpers', () => {
  it('normalizes profile display fields with Chinese fallbacks', () => {
    expect(
      getProfileDisplayName({
        username: 'admin',
        displayName: '管理员',
      }),
    ).toBe('管理员')

    expect(getProfileDisplayName({ username: 'admin' })).toBe('admin')
    expect(getProfileDisplayName(null)).toBe('未登录')
    expect(getProfileRole({ username: 'admin' })).toBe('管理员')
    expect(getProfileAccountId({ id: 7 })).toBe('7')
    expect(getProfileAccountId({ ID: 8 })).toBe('8')
    expect(getProfileAccountId({ username: 'admin' })).toBe('暂无数据')
    expect(getProfileCreatedAt({ createdAt: '2026-06-26T10:00:00Z' })).toBe('2026-06-26T10:00:00Z')
    expect(getProfileCreatedAt({ created_at: '2026-06-26T10:00:00Z' })).toBe('2026-06-26T10:00:00Z')
    expect(getProfileCreatedAt({ username: 'admin' })).toBe('暂无数据')
  })

  it('treats whitespace-only profile metadata as unavailable', () => {
    expect(
      getProfileDisplayName({
        displayName: '   ',
        username: '   ',
        name: '   ',
      }),
    ).toBe('未登录')
    expect(getProfileRole({ role: '   ' })).toBe('管理员')
    expect(getProfileCreatedAt({ createdAt: '   ', created_at: '   ' })).toBe('暂无数据')
  })

  it('validates and normalizes new password input', () => {
    expect(validateNewPassword('')).toBe('请输入新密码')
    expect(validateNewPassword('  ')).toBe('请输入新密码')
    expect(validateNewPassword('12345')).toBe('新密码至少需要 6 个字符')
    expect(validateNewPassword('123456')).toBeNull()
    expect(normalizeNewPassword('  123456  ')).toBe('123456')
  })
})

describe('user profile page', () => {
  it('renders unavailable account metadata as no data', async () => {
    mountUserProfilePage()
    useAuthStore().user = {
      username: 'admin',
    }
    await flushPromises()

    expect(wrapper?.text()).toContain('admin')
    expect(wrapper?.text()).toContain('暂无数据')
    expect(wrapper?.text()).not.toContain('待接入')
  })

  it('renders whitespace-only account metadata with Chinese fallbacks', async () => {
    mountUserProfilePage()
    useAuthStore().user = {
      username: '   ',
      displayName: '   ',
      role: '   ',
      createdAt: '   ',
    }
    await flushPromises()

    expect(wrapper?.text()).toContain('未登录')
    expect(wrapper?.text()).toContain('管理员')
    expect(wrapper?.text()).toContain('暂无数据')
  })

  it('blocks short passwords before calling the API', async () => {
    mountUserProfilePage()
    useAuthStore().user = {
      username: 'admin',
    }
    await flushPromises()

    await wrapper
      ?.find<HTMLInputElement>('[data-test="new-password-input"] input')
      .setValue('12345')
    await wrapper?.find('button').trigger('click')
    await flushPromises()

    expect(changePassword).not.toHaveBeenCalled()
    expect(ElMessage.error).toHaveBeenCalledWith('新密码至少需要 6 个字符')
  })

  it('submits a trimmed valid password and clears the input', async () => {
    mountUserProfilePage()
    useAuthStore().user = {
      username: 'admin',
    }
    await flushPromises()

    const input = wrapper?.find<HTMLInputElement>('[data-test="new-password-input"] input')

    await input?.setValue('  new-password-123  ')
    await wrapper?.find('button').trigger('click')
    await flushPromises()

    expect(changePassword).toHaveBeenCalledWith({
      newPassword: 'new-password-123',
    })
    expect(input?.element.value).toBe('')
  })
})
