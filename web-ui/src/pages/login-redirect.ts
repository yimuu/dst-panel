import { routes } from '@/shared/config/routes'

const blockedRedirects: Set<string> = new Set([routes.login, routes.init])

export function resolveLoginRedirect(value: unknown): string {
  if (typeof value !== 'string' || !value.startsWith('/')) {
    return routes.panel
  }

  const path = value.split(/[?#]/, 1)[0] || value
  if (value.startsWith('//') || blockedRedirects.has(path)) {
    return routes.panel
  }

  return value
}
