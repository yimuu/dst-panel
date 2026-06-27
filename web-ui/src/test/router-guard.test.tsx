import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import App from '@/app/App'
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

  it('redirects anonymous users away from protected routes', async () => {
    window.location.hash = routes.panel
    window.sessionStorage.clear()

    render(<App />)

    expect(await screen.findByRole('button', { name: /登\s*录/ })).toBeInTheDocument()
    expect(screen.queryByText('服务器控制面板加载中')).not.toBeInTheDocument()
  })
})
