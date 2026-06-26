import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'

import AdminLayout from '@/layouts/AdminLayout.vue'
import AuthLayout from '@/layouts/AuthLayout.vue'
import BackupPage from '@/pages/BackupPage.vue'
import DashboardPage from '@/pages/DashboardPage.vue'
import FeatureUnavailablePage from '@/pages/FeatureUnavailablePage.vue'
import HelpPage from '@/pages/HelpPage.vue'
import InitPage from '@/pages/InitPage.vue'
import LobbyPage from '@/pages/LobbyPage.vue'
import LoginPage from '@/pages/LoginPage.vue'
import ModPage from '@/pages/ModPage.vue'
import PanelPage from '@/pages/PanelPage.vue'
import PlayerLogPage from '@/pages/PlayerLogPage.vue'
import SettingsPage from '@/pages/SettingsPage.vue'
import UserProfilePage from '@/pages/UserProfilePage.vue'
import WorldLevelsPage from '@/pages/WorldLevelsPage.vue'
import { getInitStatus } from '@/features/auth/auth.api'
import { isApiSuccess } from '@/shared/api/http'
import { routes as appRoutes } from '@/shared/config/routes'
import { useAuthStore } from '@/shared/stores/auth'

const authRoutes: RouteRecordRaw[] = [
  {
    path: appRoutes.login.slice(1),
    name: 'login',
    component: LoginPage,
    meta: { public: true },
  },
  {
    path: appRoutes.init.slice(1),
    name: 'init',
    component: InitPage,
    meta: { public: true },
  },
]

const adminRoutes: RouteRecordRaw[] = [
  {
    path: appRoutes.dashboard.slice(1),
    name: 'dashboard',
    component: DashboardPage,
  },
  {
    path: appRoutes.panel.slice(1),
    name: 'panel',
    component: PanelPage,
  },
  {
    path: appRoutes.clusterIni.slice(1),
    name: 'home-clusterIni',
    component: FeatureUnavailablePage,
  },
  {
    path: appRoutes.adminlist.slice(1),
    name: 'home-adminlist',
    component: FeatureUnavailablePage,
  },
  {
    path: appRoutes.whitelist.slice(1),
    name: 'home-whitelist',
    component: FeatureUnavailablePage,
  },
  {
    path: appRoutes.blacklist.slice(1),
    name: 'home-blacklist',
    component: FeatureUnavailablePage,
  },
  {
    path: appRoutes.levels.slice(1),
    name: 'levels-levels',
    component: WorldLevelsPage,
  },
  {
    path: appRoutes.selectorMod.slice(1),
    name: 'levels-selectorMod',
    component: FeatureUnavailablePage,
  },
  {
    path: appRoutes.preinstall.slice(1),
    name: 'levels-preinstall',
    component: FeatureUnavailablePage,
  },
  {
    path: appRoutes.genMap.slice(1),
    name: 'levels-genMap',
    component: FeatureUnavailablePage,
  },
  {
    path: appRoutes.mod.slice(1),
    name: 'mod',
    component: ModPage,
  },
  {
    path: appRoutes.backup.slice(1),
    name: 'backup',
    component: BackupPage,
  },
  {
    path: appRoutes.playerLog.slice(1),
    name: 'playerLog',
    component: PlayerLogPage,
  },
  {
    path: appRoutes.setting.slice(1),
    name: 'setting',
    component: SettingsPage,
  },
  {
    path: appRoutes.lobby.slice(1),
    name: 'lobby',
    component: LobbyPage,
  },
  {
    path: appRoutes.help.slice(1),
    name: 'help',
    component: HelpPage,
  },
  {
    path: appRoutes.userProfile.slice(1),
    name: 'userProfile',
    component: UserProfilePage,
  },
]

const routeRecords: RouteRecordRaw[] = [
  {
    path: '/',
    redirect: appRoutes.panel,
  },
  {
    path: '/',
    component: AuthLayout,
    children: authRoutes,
  },
  {
    path: '/',
    component: AdminLayout,
    children: adminRoutes,
  },
]

export function createAppRouter() {
  const router = createRouter({
    history: createWebHashHistory(),
    routes: routeRecords,
  })

  router.beforeEach(async (to) => {
    const firstRun = await checkFirstRun()

    if (firstRun && to.path !== appRoutes.init) {
      return {
        path: appRoutes.init,
        query: {
          redirect: to.fullPath,
        },
      }
    }

    if (!firstRun && to.path === appRoutes.init) {
      return appRoutes.login
    }

    if (to.meta.public) {
      return true
    }

    const auth = useAuthStore()

    if (!auth.initialized) {
      await auth.fetchCurrentUser()
    }

    if (!auth.isAuthenticated) {
      return {
        path: appRoutes.login,
        query: {
          redirect: to.fullPath,
        },
      }
    }

    return true
  })

  return router
}

async function checkFirstRun(): Promise<boolean> {
  try {
    const response = await getInitStatus()
    return isApiSuccess(response)
  } catch {
    return false
  }
}
