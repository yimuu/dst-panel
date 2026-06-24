import { createApp } from 'vue'

import '@/shared/styles/main.css'

import App from './App.vue'
import { installProviders } from './providers'
import { createAppRouter } from './router'

const app = createApp(App)

installProviders(app)
app.use(createAppRouter())

app.mount('#app')
