import { describe, expect, it } from 'vitest'

import { getAuthRedirect } from '@/features/auth/auth-state'
import { routes } from '@/shared/config/routes'

describe('auth route decisions', () => {
  it('sends first-run users to init', () => {
    expect(
      getAuthRedirect({
        firstRun: true,
        authenticated: false,
        publicRoute: false,
        path: routes.panel,
      }),
    ).toBe(routes.init)
  })

  it('sends anonymous protected users to login', () => {
    expect(
      getAuthRedirect({
        firstRun: false,
        authenticated: false,
        publicRoute: false,
        path: routes.panel,
      }),
    ).toBe(routes.login)
  })

  it('allows authenticated protected routes', () => {
    expect(
      getAuthRedirect({
        firstRun: false,
        authenticated: true,
        publicRoute: false,
        path: routes.panel,
      }),
    ).toBeUndefined()
  })
})
