import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
    plugins: [
        vue(),
        vuetify({
            autoImport: false, // Disable auto-import for better tree-shaking
        }),
    ],
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
        rollupOptions: {
            output: {
                manualChunks: {
                    // Separate vendor chunks
                    'vue-vendor': ['vue', 'vue-router'],
                    'vuetify-vendor': ['vuetify'],
                    icons: ['lucide-vue-next'],
                },
            },
        },
        // Enable CSS code splitting
        cssCodeSplit: true,
        // Optimize chunk size
        chunkSizeWarningLimit: 600,
    },
    // Configure for SPA routing in production
    preview: {
        port: 80,
        host: '0.0.0.0',
    },
})
