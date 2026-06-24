import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { describe, expect, it } from 'vitest'

import { routes } from '@/shared/config/routes'
import { useAuthStore } from '@/shared/stores/auth'

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

  it('shows the personal profile route label in the admin shell', async () => {
    const pinia = createPinia()
    setActivePinia(pinia)
    const router = createAppRouter()
    const auth = useAuthStore()
    auth.user = { username: 'admin' }
    auth.initialized = true

    await router.push(routes.userProfile)
    await router.isReady()

    const wrapper = mount(App, {
      global: {
        plugins: [pinia, router],
      },
    })

    expect(wrapper.find('.admin-header__title').text()).toBe('个人信息')
  })
})
