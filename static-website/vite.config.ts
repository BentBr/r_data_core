import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
    plugins: [vue(), vuetify({ autoImport: true })],
    resolve: {
        alias: {
            '@': fileURLToPath(new URL('./src', import.meta.url)),
        },
    },
    server: {
        port: 80,
        strictPort: true,
        host: '0.0.0.0',
        allowedHosts: ['website.rdatacore.docker', 'localhost'],
    },
    build: {
        target: 'esnext',
        sourcemap: true,
        outDir: 'dist',
    },
    // Configure for SPA routing in production
    preview: {
        port: 80,
        host: '0.0.0.0',
    },
})
