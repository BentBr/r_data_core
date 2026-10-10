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
        exclude: ['**/e2e/**', '**/node_modules/**'],
        server: {
            deps: {
                inline: ['vuetify'],
            },
        },
        coverage: {
            provider: 'v8',
            reporter: ['text', 'json-summary', 'lcov'],
            reportsDirectory: './coverage',
            include: ['src/**/*.{ts,vue}'],
            // Generated ts-rs bindings, the app bootstrap, ambient type
            // declarations and the tests themselves carry no testable runtime.
            // Finalized against the measurement report (plan Task 5).
            exclude: ['src/types/generated/**', 'src/main.ts', 'src/**/*.d.ts', 'src/**/*.test.ts'],
            // Lines-only gate, matching the backend's --fail-under-lines, which
            // is already 80 (MIN_LINES in scripts/coverage.sh). Branch /
            // function / statement coverage is reported but not gated.
            //
            // vitest 4's v8 provider counts 226 files where v3 counted 172,
            // so this is 80% of the real tree rather than of the flattering
            // subset v3 measured. Keep in sync with FE_THRESHOLD in
            // .github/workflows/coverage.yml — the badge there only paints
            // green at 80 or above.
            thresholds: {
                lines: 80,
            },
        },
    },
    resolve: {
        alias: {
            '@': resolve(__dirname, 'src'),
        },
    },
})
