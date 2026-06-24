import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { describe, expect, it } from 'vitest'

import { routes } from '@/shared/config/routes'

import App from '../App.vue'
import { createAppRouter } from '../router'

describe('App', () => {
  it('mounts the public auth route', async () => {
    const pinia = createPinia()
    const router = createAppRouter()

    await router.push(routes.login)
    await router.isReady()

    const wrapper = mount(App, {
      global: {
        plugins: [pinia, router],
      },
    })

    expect(wrapper.text()).toContain('登录')
  })
})
