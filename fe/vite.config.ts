import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'
import Components from 'unplugin-vue-components/vite'
import { fileURLToPath, URL } from 'node:url'
import { readFileSync, existsSync, readdirSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { VirtualVueSFCPlugin } from './vite-plugin-virtual-vue-sfc'

// Read version from package.json
const packageJson = JSON.parse(readFileSync('./package.json', 'utf-8'))
const appVersion = packageJson.version

// Custom resolver for unplugin-vue-components to find virtual index.vue components
export default defineConfig({
    plugins: [
        VirtualVueSFCPlugin(),
        vue(),
        vuetify({ autoImport: true }),
        Components({
            dirs: [
                'src/shared/components',
                'src/shared/forms',
                'src/shared/layouts',
                'src/shared/tables',
                'src/modules/**/components',
                'src/modules/**/pages',
            ],
            resolvers: [], // No custom resolver needed anymore
            include: [/\.vue$/, /\.vue\?vue/, /\.html$/],
            dts: 'src/types/components.d.ts',
        }),
    ],
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
    optimizeDeps: {
        include: ['json-editor-vue', 'vanilla-jsoneditor'],
    },
})
