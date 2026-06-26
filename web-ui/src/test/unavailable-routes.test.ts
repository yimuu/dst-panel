import { describe, expect, it } from 'vitest'

import { createAppRouter } from '@/app/router'
import ClusterIniPage from '@/pages/ClusterIniPage.vue'
import FeatureUnavailablePage from '@/pages/FeatureUnavailablePage.vue'
import PlayerListPage from '@/pages/PlayerListPage.vue'
import { routes } from '@/shared/config/routes'

describe('unfinished admin routes', () => {
  it('routes completed cluster settings to the real page', () => {
    const router = createAppRouter()

    expect(router.resolve(routes.clusterIni).matched.at(-1)?.components?.default).toBe(
      ClusterIniPage,
    )
  })

  it('routes completed player list pages to the shared real page', () => {
    const router = createAppRouter()
    const routeExpectations = [
      [routes.adminlist, 'adminlist', '管理员列表'],
      [routes.whitelist, 'whitelist', '白名单'],
      [routes.blacklist, 'blacklist', '黑名单'],
    ] as const

    for (const [path, kind, title] of routeExpectations) {
      const route = router.resolve(path).matched.at(-1)

      expect(route?.components?.default).toBe(PlayerListPage)
      expect(route?.props.default).toEqual(
        expect.objectContaining({
          kind,
          title,
        }),
      )
    }
  })

  it('route to an explicit unavailable page instead of unrelated business pages', () => {
    const router = createAppRouter()
    const unfinishedRoutes = [
      routes.selectorMod,
      routes.preinstall,
      routes.genMap,
    ]

    for (const path of unfinishedRoutes) {
      expect(router.resolve(path).matched.at(-1)?.components?.default).toBe(FeatureUnavailablePage)
    }
  })
})
