import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath } from 'node:url'

export default defineConfig({
    plugins: [vue()],
    test: {
        globals: true,
        environment: 'happy-dom',
        setupFiles: ['./src/test/setup.ts'],
        coverage: {
            provider: 'v8',
            reporter: ['text', 'json', 'html'],
            exclude: [
                'node_modules/',
                'src/test/',
                '**/*.spec.ts',
                '**/*.test.ts',
                'scripts/',
                'dist/',
            ],
        },
        css: false, // Disable CSS processing in tests
        server: {
            deps: {
                inline: ['vuetify'], // Inline vuetify to avoid CSS import issues
            },
        },
    },
    resolve: {
        alias: {
            '@': fileURLToPath(new URL('./src', import.meta.url)),
            '\\.(css|less|scss|sass)$': fileURLToPath(
                new URL('./src/test/mocks/styleMock.ts', import.meta.url)
            ),
        },
    },
})
