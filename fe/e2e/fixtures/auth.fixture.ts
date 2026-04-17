import { test as base, type Page } from '@playwright/test'
import { LoginPage } from '../page-objects/login.page'

const ADMIN_USERNAME = process.env.E2E_ADMIN_USERNAME ?? 'admin'
const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD ?? 'adminadmin'
const VIEWER_USERNAME = 'e2e_viewer_user'
const VIEWER_PASSWORD = 'e2e_viewer_password_123'

type AuthFixtures = {
    authenticatedPage: Page
    viewerPage: Page
    loginPage: LoginPage
}

export const test = base.extend<AuthFixtures>({
    authenticatedPage: async ({ page }, use) => {
        const loginPage = new LoginPage(page)
        await loginPage.goto()
        await loginPage.login(ADMIN_USERNAME, ADMIN_PASSWORD)
        await loginPage.expectRedirectToDashboard()
        // Wait for auth state to fully settle before tests interact with the page
        await page.waitForLoadState('networkidle')
        await page.getByTestId('nav-sidebar').waitFor({ state: 'visible', timeout: 15_000 })
        // Confirm permissions are loaded: nav items depend on canAccessRoute
        await page.getByTestId('nav-item-/dashboard').waitFor({ state: 'visible', timeout: 10_000 })
        await use(page)
    },

    viewerPage: async ({ browser }, use) => {
        const context = await browser.newContext()
        const page = await context.newPage()
        const loginPage = new LoginPage(page)
        await loginPage.goto()
        await loginPage.login(VIEWER_USERNAME, VIEWER_PASSWORD)
        await loginPage.expectRedirectToDashboard()
        await page.waitForLoadState('networkidle')
        await page.getByTestId('nav-sidebar').waitFor({ state: 'visible', timeout: 15_000 })
        await page.getByTestId('nav-item-/dashboard').waitFor({ state: 'visible', timeout: 10_000 })
        await use(page)
        await context.close()
    },

    loginPage: async ({ page }, use) => {
        const loginPage = new LoginPage(page)
        await use(loginPage)
    },
})

export { expect } from '@playwright/test'
