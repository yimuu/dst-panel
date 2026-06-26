import { fileURLToPath, URL } from 'node:url'

import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  publicDir: 'public',
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              name: 'vue',
              test: /node_modules[\\/](vue|vue-router|pinia)[\\/]/,
              priority: 2,
            },
            {
              name: 'element',
              test: /node_modules[\\/](element-plus|@element-plus)[\\/]/,
              priority: 1,
              maxSize: 450 * 1024,
            },
          ],
        },
      },
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://127.0.0.1:8082',
      '/steam': 'http://127.0.0.1:8082',
      '/webhook': 'http://127.0.0.1:8082',
      '/share': 'http://127.0.0.1:8082',
      '/ws': {
        target: 'ws://127.0.0.1:8082',
        ws: true,
      },
    },
  },
})
