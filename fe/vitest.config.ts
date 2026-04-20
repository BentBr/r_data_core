import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import vuetify from 'vite-plugin-vuetify'
import Components from 'unplugin-vue-components/vite'
import { resolve, join } from 'path'
import { existsSync, readdirSync } from 'node:fs'
import { VirtualVueSFCPlugin } from './vite-plugin-virtual-vue-sfc'

const rootDir = resolve(__dirname, 'src')
const searchDirs = [
    join(rootDir, 'shared/components'),
    join(rootDir, 'shared/forms'),
    join(rootDir, 'shared/layouts'),
    join(rootDir, 'shared/tables'),
]

// Add module-specific component and page directories
const modulesDir = join(rootDir, 'modules')
if (existsSync(modulesDir)) {
    const modules = readdirSync(modulesDir)
    for (const moduleName of modules) {
        const modulePath = join(modulesDir, moduleName)
        const compDir = join(modulePath, 'components')
        const pageDir = join(modulePath, 'pages')
        if (existsSync(compDir)) searchDirs.push(compDir)
        if (existsSync(pageDir)) searchDirs.push(pageDir)
    }
}

const RDataCoreResolver = (name: string) => {
    // Check each directory for a sub-folder matching the component name
    for (const dir of searchDirs) {
        const componentPath = join(dir, name)
        // If the directory exists and contains a .ts file matching its name, it's our component
        if (existsSync(componentPath) && existsSync(join(componentPath, `${name}.ts`))) {
            // Return the path relative to src, with @/ prefix
            const relativePath = componentPath.replace(rootDir, '@')
            return `${relativePath}/index.vue`
        }
    }

    // Special case for App/index.vue if needed
    if (name === 'App') {
        return '@/App/index.vue'
    }

    return null
}

export default defineConfig({
    plugins: [
        VirtualVueSFCPlugin(),
        vue(),
        vuetify({ autoImport: true }),
        Components({
            resolvers: [RDataCoreResolver],
            include: [/\.vue$/, /\.vue\?vue/, /\.html$/],
            dts: false, // Don't generate dts during tests
        }),
    ],
    define: {
        __APP_VERSION__: JSON.stringify('0.0.0-test'),
    },
    test: {
        globals: true,
        environment: 'jsdom',
        setupFiles: ['./src/test-setup.ts'],
        css: true,
        exclude: ['**/e2e/**', '**/node_modules/**'],
        server: {
            deps: {
                inline: ['vuetify'],
            },
        },
    },
    resolve: {
        alias: {
            '@': rootDir,
        },
    },
})
