import { defineComponent, h } from 'vue'
import { createRouter, createWebHashHistory, type RouteRecordRaw } from 'vue-router'

import AdminLayout from '@/layouts/AdminLayout.vue'
import AuthLayout from '@/layouts/AuthLayout.vue'
import { flattenAdminMenuItems } from '@/layouts/menu'
import { routes as appRoutes } from '@/shared/config/routes'
import { useAuthStore } from '@/shared/stores/auth'

function createPlaceholder(title: string, kind: 'admin' | 'auth' = 'admin') {
  return defineComponent({
    name: `${title}Placeholder`,
    setup() {
      return () =>
        h(
          kind === 'auth' ? 'section' : 'div',
          {
            class: kind === 'auth' ? 'auth-placeholder' : 'route-placeholder',
          },
          [h(kind === 'auth' ? 'h1' : 'h2', title), h('p', '页面建设中')],
        )
    },
  })
}

const authRoutes: RouteRecordRaw[] = [
  {
    path: appRoutes.login.slice(1),
    name: 'login',
    component: createPlaceholder('登录', 'auth'),
    meta: { public: true },
  },
  {
    path: appRoutes.init.slice(1),
    name: 'init',
    component: createPlaceholder('初始化', 'auth'),
    meta: { public: true },
  },
]

const adminRoutes: RouteRecordRaw[] = flattenAdminMenuItems().map((item) => ({
  path: item.path.slice(1),
  name: item.path.slice(1).split('/').join('-'),
  component: createPlaceholder(item.label),
}))

adminRoutes.push({
  path: appRoutes.userProfile.slice(1),
  name: 'userProfile',
  component: createPlaceholder('个人信息'),
})

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

export const router = createAppRouter()
