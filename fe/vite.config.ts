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
            '/api': {
                target: 'http://rdatacore.docker',
                changeOrigin: true,
                secure: false
            },
            '/admin/api': {
                target: 'http://rdatacore.docker',
                changeOrigin: true,
                secure: false
            }
        }
    },
    build: {
        target: 'esnext',
        sourcemap: true
    }
})
