import { describe, expect, it } from 'vitest'

import { createAppRouter } from '@/app/router'
import ClusterIniPage from '@/pages/ClusterIniPage.vue'
import FeatureUnavailablePage from '@/pages/FeatureUnavailablePage.vue'
import MapPreviewPage from '@/pages/MapPreviewPage.vue'
import PlayerListPage from '@/pages/PlayerListPage.vue'
import PreinstallPage from '@/pages/PreinstallPage.vue'
import WorldModSelectionPage from '@/pages/WorldModSelectionPage.vue'
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

  it('routes completed world mod selection to the real page', () => {
    const router = createAppRouter()

    expect(router.resolve(routes.selectorMod).matched.at(-1)?.components?.default).toBe(
      WorldModSelectionPage,
    )
  })

  it('routes completed preinstall template page to the real page', () => {
    const router = createAppRouter()

    expect(router.resolve(routes.preinstall).matched.at(-1)?.components?.default).toBe(
      PreinstallPage,
    )
  })

  it('routes completed map preview page to the real page', () => {
    const router = createAppRouter()

    expect(router.resolve(routes.genMap).matched.at(-1)?.components?.default).toBe(MapPreviewPage)
  })

  it('does not leave rebuilt workflow routes on the unavailable page', () => {
    const router = createAppRouter()
    const rebuiltRoutes = [
      routes.clusterIni,
      routes.adminlist,
      routes.whitelist,
      routes.blacklist,
      routes.selectorMod,
      routes.preinstall,
      routes.genMap,
    ]

    for (const path of rebuiltRoutes) {
      expect(router.resolve(path).matched.at(-1)?.components?.default).not.toBe(
        FeatureUnavailablePage,
      )
    }
  })
})
