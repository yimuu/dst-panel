import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import App from '../App.vue'

describe('App', () => {
  it('mounts the DST Admin shell', () => {
    const wrapper = mount(App)

    expect(wrapper.text()).toContain('DST Admin')
  })
})
