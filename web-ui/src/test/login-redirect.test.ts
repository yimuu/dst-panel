import { describe, expect, it } from 'vitest'

import { resolveLoginRedirect } from '@/pages/login-redirect'
import { routes } from '@/shared/config/routes'

describe('login redirect resolution', () => {
  it('keeps internal protected redirects', () => {
    expect(resolveLoginRedirect('/panel?tab=levels')).toBe('/panel?tab=levels')
  })

  it('falls back when redirect is external or points back to public auth routes', () => {
    expect(resolveLoginRedirect('https://example.com')).toBe(routes.panel)
    expect(resolveLoginRedirect('//example.com')).toBe(routes.panel)
    expect(resolveLoginRedirect(routes.login)).toBe(routes.panel)
    expect(resolveLoginRedirect(`${routes.login}?redirect=/setting`)).toBe(routes.panel)
    expect(resolveLoginRedirect(routes.init)).toBe(routes.panel)
  })
})
