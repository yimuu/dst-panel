import { describe, expect, it } from 'vitest'

import {
  getProfileAccountId,
  getProfileCreatedAt,
  getProfileDisplayName,
  getProfileRole,
  normalizeNewPassword,
  validateNewPassword,
} from '@/features/auth/user-profile'

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
    expect(getProfileCreatedAt({ createdAt: '2026-06-26T10:00:00Z' })).toBe(
      '2026-06-26T10:00:00Z',
    )
    expect(getProfileCreatedAt({ created_at: '2026-06-26T10:00:00Z' })).toBe(
      '2026-06-26T10:00:00Z',
    )
    expect(getProfileCreatedAt({ username: 'admin' })).toBe('暂无数据')
  })

  it('validates and normalizes new password input', () => {
    expect(validateNewPassword('')).toBe('请输入新密码')
    expect(validateNewPassword('  ')).toBe('请输入新密码')
    expect(validateNewPassword('12345')).toBe('新密码至少需要 6 个字符')
    expect(validateNewPassword('123456')).toBeNull()
    expect(normalizeNewPassword('  123456  ')).toBe('123456')
  })
})
