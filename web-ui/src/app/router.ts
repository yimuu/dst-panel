import { h } from 'vue'
import { createRouter, createWebHashHistory } from 'vue-router'

const ScaffoldView = {
  name: 'ScaffoldView',
  setup() {
    return () =>
      h('main', { class: 'app-shell' }, [
        h('h1', 'DST Admin'),
        h('p', 'Vue frontend scaffold is ready.'),
      ])
  },
}

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      name: 'scaffold',
      component: ScaffoldView,
    },
  ],
})
