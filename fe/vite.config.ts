import {defineConfig} from 'vite'
import vue from '@vitejs/plugin-vue'
import {fileURLToPath, URL} from 'node:url'

export default defineConfig({
    plugins: [
        vue(),
    ],
    resolve: {
        alias: {
            '@': fileURLToPath(new URL('./src', import.meta.url))
        }
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
                secure: false
            },
            '/admin/api/v1': {
                target: 'http://rdatacore.docker',
                changeOrigin: true,
                secure: false
            }
        }
    },
    // Configure for SPA routing in production
    preview: {
        port: 80,
        host: '0.0.0.0',
    },
    build: {
        target: 'esnext',
        sourcemap: true
    }
})
