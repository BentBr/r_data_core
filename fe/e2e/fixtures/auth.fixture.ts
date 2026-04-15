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
        await use(page)
    },

    viewerPage: async ({ browser }, use) => {
        const context = await browser.newContext()
        const page = await context.newPage()
        const loginPage = new LoginPage(page)
        await loginPage.goto()
        await loginPage.login(VIEWER_USERNAME, VIEWER_PASSWORD)
        await loginPage.expectRedirectToDashboard()
        await use(page)
        await context.close()
    },

    loginPage: async ({ page }, use) => {
        const loginPage = new LoginPage(page)
        await use(loginPage)
    },
})

export { expect } from '@playwright/test'
