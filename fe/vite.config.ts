import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'
import { fileURLToPath, URL } from 'node:url'
import { readFileSync } from 'node:fs'

// Read version from package.json
const packageJson = JSON.parse(readFileSync('./package.json', 'utf-8'))
const appVersion = packageJson.version

export default defineConfig({
    plugins: [vue(), vuetify({ autoImport: true })],
    define: {
        __APP_VERSION__: JSON.stringify(appVersion),
    },
    resolve: {
        alias: {
            '@': fileURLToPath(new URL('./src', import.meta.url)),
        },
    },
    server: {
        port: 80,
        strictPort: true,
        host: '0.0.0.0',
        allowedHosts: ['admin.rdatacore.docker', 'localhost'],
        proxy: {
            // Specific API endpoints - avoid conflicts with frontend routes like /api-keys
            '/api/v1': {
                target: 'http://rdatacore.docker',
                changeOrigin: true,
                secure: false,
            },
            '/admin/api/v1': {
                target: 'http://rdatacore.docker',
                changeOrigin: true,
                secure: false,
            },
        },
    },
    // Configure for SPA routing in production
    preview: {
        port: 80,
        host: '0.0.0.0',
    },
    build: {
        target: 'esnext',
        sourcemap: true,
    },
})
