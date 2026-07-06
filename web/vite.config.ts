/// <reference types="vitest/config" />
import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  return {
    plugins: [react(), tailwindcss()],
    server: {
      proxy: {
        // In dev, proxy GraphQL to the actix server (`tunein web`).
        // Override the target with TUNEIN_API_URL, or point the client
        // elsewhere entirely with VITE_API_URL.
        '/graphql': {
          target: env.TUNEIN_API_URL ?? 'http://localhost:8881',
          changeOrigin: true,
        },
      },
    },
    test: {
      environment: 'happy-dom',
      setupFiles: './src/test/setup.ts',
    },
  }
})
