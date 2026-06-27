import { fileURLToPath, URL } from 'node:url'

import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:8082',
      '/ws': {
        target: 'ws://127.0.0.1:8082',
        ws: true,
      },
      '/steam': 'http://127.0.0.1:8082',
      '/webhook': 'http://127.0.0.1:8082',
      '/share': 'http://127.0.0.1:8082',
    },
  },
  build: {
    outDir: '../dist',
    emptyOutDir: true,
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
  },
})
