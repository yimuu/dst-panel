import { describe, expect, it } from 'vitest'

import { normalizeUserProfileName, validateNewPassword } from '@/features/auth/user-profile'

describe('user profile helpers', () => {
  it('normalizes names and password input', () => {
    expect(normalizeUserProfileName({ displayName: 'admin' })).toBe('admin')
    expect(normalizeUserProfileName({ username: 'root' })).toBe('root')
    expect(validateNewPassword(' 12345 ')).toBe('密码长度至少 6 位')
    expect(validateNewPassword(' 123456 ')).toBeUndefined()
  })
})
