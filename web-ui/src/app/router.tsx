import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { createHashRouter, Navigate, Outlet, RouterProvider, useLocation } from 'react-router'

import AdminLayout from '@/layouts/AdminLayout'
import AuthLayout from '@/layouts/AuthLayout'
import BackupPage from '@/pages/BackupPage'
import ClusterIniPage from '@/pages/ClusterIniPage'
import DashboardPage from '@/pages/DashboardPage'
import HelpPage from '@/pages/HelpPage'
import InitPage from '@/pages/InitPage'
import LobbyPage from '@/pages/LobbyPage'
import LoginPage from '@/pages/LoginPage'
import MapPreviewPage from '@/pages/MapPreviewPage'
import ModPage from '@/pages/ModPage'
import PanelPage from '@/pages/PanelPage'
import PlayerListPage from '@/pages/PlayerListPage'
import PlayerLogPage from '@/pages/PlayerLogPage'
import PreinstallPage from '@/pages/PreinstallPage'
import SettingsPage from '@/pages/SettingsPage'
import UserProfilePage from '@/pages/UserProfilePage'
import WorldLevelsPage from '@/pages/WorldLevelsPage'
import WorldModSelectionPage from '@/pages/WorldModSelectionPage'
import { getCurrentUser, getInitStatus } from '@/features/auth/auth.api'
import {
  clearAuthRouteState,
  getAuthRedirect,
  readAuthRouteState,
} from '@/features/auth/auth-state'
import { isApiSuccess } from '@/shared/api/envelope'
import { routes } from '@/shared/config/routes'

function RouteGuard({ publicRoute = false }: { publicRoute?: boolean }) {
  const location = useLocation()
  const localState = readAuthRouteState()
  const initQuery = useQuery({
    queryKey: ['auth', 'init-status'],
    queryFn: getInitStatus,
  })
  const firstRun = initQuery.isSuccess ? isApiSuccess(initQuery.data) : localState.firstRun
  const initResolved = initQuery.isSuccess || initQuery.isError
  const shouldVerifySession = !publicRoute && initResolved && !firstRun
  const userQuery = useQuery({
    queryKey: ['auth', 'current-user'],
    queryFn: getCurrentUser,
    enabled: shouldVerifySession,
  })

  const authenticated = shouldVerifySession
    ? userQuery.isSuccess && isApiSuccess(userQuery.data)
    : false

  useEffect(() => {
    if (shouldVerifySession && !userQuery.isPending && !authenticated) {
      clearAuthRouteState()
    }
  }, [authenticated, shouldVerifySession, userQuery.isPending])

  if (initQuery.isPending || (shouldVerifySession && userQuery.isPending)) {
    return <div className="route-loading">加载中</div>
  }

  const redirect = getAuthRedirect({
    firstRun,
    authenticated,
    publicRoute,
    path: location.pathname,
  })

  if (redirect) {
    return <Navigate to={redirect} replace />
  }

  return <Outlet />
}

export const router = createHashRouter([
  {
    element: <RouteGuard publicRoute />,
    children: [
      {
        element: <AuthLayout />,
        children: [
          { path: routes.login, element: <LoginPage /> },
          { path: routes.init, element: <InitPage /> },
        ],
      },
    ],
  },
  {
    element: <RouteGuard />,
    children: [
      {
        element: <AdminLayout />,
        children: [
          { index: true, element: <Navigate to={routes.panel} replace /> },
          { path: routes.dashboard, element: <DashboardPage /> },
          { path: routes.panel, element: <PanelPage /> },
          { path: routes.clusterIni, element: <ClusterIniPage /> },
          { path: routes.adminlist, element: <PlayerListPage title="管理员列表" /> },
          { path: routes.whitelist, element: <PlayerListPage title="白名单列表" /> },
          { path: routes.blacklist, element: <PlayerListPage title="黑名单列表" /> },
          { path: routes.levels, element: <WorldLevelsPage /> },
          { path: routes.selectorMod, element: <WorldModSelectionPage /> },
          { path: routes.preinstall, element: <PreinstallPage /> },
          { path: routes.genMap, element: <MapPreviewPage /> },
          { path: routes.mod, element: <ModPage /> },
          { path: routes.backup, element: <BackupPage /> },
          { path: routes.playerLog, element: <PlayerLogPage /> },
          { path: routes.setting, element: <SettingsPage /> },
          { path: routes.lobby, element: <LobbyPage /> },
          { path: routes.help, element: <HelpPage /> },
          { path: routes.userProfile, element: <UserProfilePage /> },
        ],
      },
    ],
  },
  { path: '*', element: <Navigate to={routes.panel} replace /> },
])

export function AppRouter() {
  return <RouterProvider router={router} />
}
