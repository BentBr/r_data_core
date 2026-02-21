import { defineConfig } from '@playwright/test'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const baseURL = process.env.E2E_BASE_URL ?? 'http://admin.rdatacore.docker'

export default defineConfig({
    testDir: path.resolve(__dirname, 'tests'),
    outputDir: path.resolve(__dirname, 'test-results'),
    fullyParallel: false,
    workers: 1,
    retries: process.env.CI ? 1 : 0,
    reporter: process.env.CI
        ? [
              [
                  'html',
                  { outputFolder: path.resolve(__dirname, 'playwright-report'), open: 'never' },
              ],
              ['github'],
          ]
        : [
              [
                  'html',
                  {
                      outputFolder: path.resolve(__dirname, 'playwright-report'),
                      open: 'on-failure',
                  },
              ],
          ],
    globalSetup: path.resolve(__dirname, 'global-setup.ts'),
    globalTeardown: path.resolve(__dirname, 'global-teardown.ts'),
    use: {
        baseURL,
        trace: 'on-first-retry',
        video: 'on-first-retry',
        screenshot: 'only-on-failure',
        actionTimeout: 10_000,
        navigationTimeout: 15_000,
    },
    projects: [
        {
            name: 'chromium',
            use: {
                browserName: 'chromium',
                viewport: { width: 1440, height: 900 },
            },
        },
    ],
    expect: {
        timeout: 10_000,
    },
})
