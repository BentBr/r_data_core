import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'
import { fileURLToPath, URL } from 'node:url'
import { deferCssPlugin } from './vite-plugin-defer-css'

export default defineConfig(({ isSsrBuild }) => ({
    plugins: [
        vue(),
        vuetify({
            autoImport: false, // Disable auto-import for better tree-shaking
        }),
        deferCssPlugin(),
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
            // Only apply manualChunks for client builds, not SSR
            output: isSsrBuild
                ? {}
                : {
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
    // SSR configuration - bundle Vuetify to avoid CSS import issues in Node.js
    ssr: {
        noExternal: ['vuetify', '@vueuse/head', '@vueuse/core', 'lucide-vue-next'],
    },
    // Configure for SPA routing in production
    preview: {
        port: 80,
        host: '0.0.0.0',
    },
    // SSG configuration
    ssgOptions: {
        // Pre-render all language routes
        includedRoutes: paths => {
            // Define all routes to pre-render
            const languages = ['en', 'de']
            const pages = [
                '',
                '/about',
                '/pricing',
                '/roadmap',
                '/use-cases',
                '/imprint',
                '/privacy',
            ]

            const routes: string[] = []
            for (const lang of languages) {
                for (const page of pages) {
                    routes.push(`/${lang}${page}`)
                }
            }

            // Also include the root redirect
            routes.push('/')

            return routes
        },
        // Use happy-dom for SSR
        mock: true,
        // Format HTML output
        formatting: 'minify',
        // Handle errors gracefully
        onPageRendered: (route, html) => {
            console.log(`Pre-rendered: ${route}`)
            return html
        },
        // Critical CSS extraction - disabled for now to ensure full CSS loads
        // The 'swap' preload wasn't adding the CSS link properly
        beastiesOptions: false,
    },
}))
