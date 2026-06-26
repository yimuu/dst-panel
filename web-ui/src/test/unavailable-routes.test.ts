import { describe, expect, it } from 'vitest'

import { createAppRouter } from '@/app/router'
import ClusterIniPage from '@/pages/ClusterIniPage.vue'
import FeatureUnavailablePage from '@/pages/FeatureUnavailablePage.vue'
import { routes } from '@/shared/config/routes'

describe('unfinished admin routes', () => {
  it('routes completed cluster settings to the real page', () => {
    const router = createAppRouter()

    expect(router.resolve(routes.clusterIni).matched.at(-1)?.components?.default).toBe(
      ClusterIniPage,
    )
  })

  it('route to an explicit unavailable page instead of unrelated business pages', () => {
    const router = createAppRouter()
    const unfinishedRoutes = [
      routes.adminlist,
      routes.whitelist,
      routes.blacklist,
      routes.selectorMod,
      routes.preinstall,
      routes.genMap,
    ]

    for (const path of unfinishedRoutes) {
      expect(router.resolve(path).matched.at(-1)?.components?.default).toBe(FeatureUnavailablePage)
    }
  })
})
