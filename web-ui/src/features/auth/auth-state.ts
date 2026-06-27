import { routes } from '@/shared/config/routes'

const AUTHENTICATED_KEY = 'dst-admin.authenticated'
const FIRST_RUN_KEY = 'dst-admin.firstRun'

export interface AuthRouteDecision {
  firstRun: boolean
  authenticated: boolean
  publicRoute: boolean
  path: string
}

export interface AuthRouteState {
  firstRun: boolean
  authenticated: boolean
}

function getSessionStorage(): Storage | undefined {
  return typeof window === 'undefined' ? undefined : window.sessionStorage
}

export function readAuthRouteState(storage = getSessionStorage()): AuthRouteState {
  return {
    firstRun: storage?.getItem(FIRST_RUN_KEY) === 'true',
    authenticated: storage?.getItem(AUTHENTICATED_KEY) === 'true',
  }
}

export function setAuthRouteState(nextState: AuthRouteState, storage = getSessionStorage()): void {
  if (!storage) {
    return
  }

  storage.setItem(FIRST_RUN_KEY, String(nextState.firstRun))
  storage.setItem(AUTHENTICATED_KEY, String(nextState.authenticated))
}

export function markAuthenticated(): void {
  setAuthRouteState({ firstRun: false, authenticated: true })
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
