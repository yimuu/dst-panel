import { routes } from '@/shared/config/routes'

export interface AuthRouteDecision {
  firstRun: boolean
  authenticated: boolean
  publicRoute: boolean
  path: string
}

export function getAuthRedirect(decision: AuthRouteDecision): string | undefined {
  if (decision.firstRun && decision.path !== routes.init) {
    return routes.init
  }

  if (!decision.firstRun && decision.path === routes.init) {
    return routes.login
  }

  if (decision.publicRoute) {
    return undefined
  }

  return decision.authenticated ? undefined : routes.login
}
