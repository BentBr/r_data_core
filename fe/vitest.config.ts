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
            // Lines-only gate, matching the backend's --fail-under-lines. Branch
            // / function / statement coverage is reported but not gated.
            //
            // 50, not the 70 this was under vitest 3. The number did not drop
            // because anything got worse: vitest 4's v8 provider counts 226
            // files where v3 counted 172, and the 54 it adds are untested Vue
            // components v3 silently skipped. Measured coverage went 74.27% ->
            // 52.5% on an unchanged test suite, so the old gate was partly
            // being met by not looking.
            //
            // This is a floor to ratchet upward as those components get
            // tests, not a target. Keep it in sync with FE_THRESHOLD in
            // .github/workflows/coverage.yml.
            thresholds: {
                lines: 50,
            },
        },
    },
    resolve: {
        alias: {
            '@': resolve(__dirname, 'src'),
        },
    },
})
