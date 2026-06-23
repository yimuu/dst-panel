import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import { createPinia } from 'pinia'
import type { App } from 'vue'

export function installProviders(app: App): void {
  app.use(createPinia())
  app.use(ElementPlus)
}
