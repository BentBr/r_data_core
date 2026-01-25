import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'
import { resolve } from 'path'

export default defineConfig({
    plugins: [vue(), vuetify({ autoImport: true })],
    define: {
        __APP_VERSION__: JSON.stringify('0.0.0-test'),
    },
    test: {
        globals: true,
        environment: 'jsdom',
        setupFiles: ['./src/test-setup.ts'],
        css: true,
        server: {
            deps: {
                inline: ['vuetify'],
            },
        },
    },
    resolve: {
        alias: {
            '@': resolve(__dirname, 'src'),
        },
    },
})
