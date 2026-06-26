import type { UserProfile } from '@/shared/types/domain'

const emptyText = '暂无数据'

export function getProfileDisplayName(user: UserProfile | null): string {
  return (
    readString(user?.displayName) ||
    readString(user?.username) ||
    readString(user?.name) ||
    '未登录'
  )
}

export function getProfileRole(user: UserProfile | null): string {
  return readString(user?.role) || '管理员'
}

export function getProfileAccountId(user: UserProfile | null): string {
  const id = user?.id ?? user?.ID
  return typeof id === 'number' || typeof id === 'string' ? String(id) : emptyText
}

export function getProfileCreatedAt(user: UserProfile | null): string {
  return readString(user?.createdAt) || readString(user?.created_at) || emptyText
}

export function normalizeNewPassword(value: string): string {
  return value.trim()
}

export function validateNewPassword(value: string): string | null {
  const password = normalizeNewPassword(value)

  if (!password) {
    return '请输入新密码'
  }

  if (password.length < 6) {
    return '新密码至少需要 6 个字符'
  }

  return null
}

function readString(value: unknown): string {
  if (typeof value !== 'string') {
    return ''
  }

  const normalized = value.trim()
  return normalized.length > 0 ? normalized : ''
}
