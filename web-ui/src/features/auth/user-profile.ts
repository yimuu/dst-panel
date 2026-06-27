import type { CurrentUser } from '@/shared/types/domain'

export function normalizeUserProfileName(user: CurrentUser): string {
  const displayName = user.displayName?.trim()
  if (displayName) {
    return displayName
  }

  const username = user.username?.trim()
  return username || '管理员'
}

export function validateNewPassword(value: string): string | undefined {
  const password = value.trim()
  if (!password) {
    return undefined
  }

  return password.length < 6 ? '密码长度至少 6 位' : undefined
}
