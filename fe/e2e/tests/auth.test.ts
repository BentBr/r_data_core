import { test, expect } from '../fixtures/auth.fixture'

const ADMIN_USERNAME = process.env.E2E_ADMIN_USERNAME ?? 'admin'
const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD ?? 'adminadmin'

test.describe('Authentication', () => {
    test('successful login redirects to dashboard', async ({ loginPage }) => {
        await loginPage.goto()
        await loginPage.login(ADMIN_USERNAME, ADMIN_PASSWORD)
        await loginPage.expectRedirectToDashboard()
    })

    test('invalid credentials show error message', async ({ loginPage, page }) => {
        await loginPage.goto()
        await loginPage.login('invalid_user', 'wrong_password_123')

        // Either the error alert appears or a field-specific error is shown
        const errorAlert = page.getByTestId('login-error')
        const fieldErrors = page.locator('.v-messages__message')
        await expect(errorAlert.or(fieldErrors.first())).toBeVisible({ timeout: 10_000 })
    })

    test('logout redirects to login page', async ({ authenticatedPage }) => {
        // Click the user profile menu button (contains the username text)
        const profileMenu = authenticatedPage.locator('.v-app-bar').getByText('admin').first()
        await profileMenu.click()

        // Vuetify renders menus in a teleported overlay — find the logout item there
        const logoutItem = authenticatedPage.locator('.v-overlay--active .v-list-item.text-error')
        await logoutItem.waitFor({ state: 'visible', timeout: 5_000 })
        await logoutItem.click()

        // Should be redirected to login
        await expect(authenticatedPage).toHaveURL(/\/login/, { timeout: 10_000 })
    })

    test('unauthenticated access redirects to login', async ({ page }) => {
        await page.goto('/dashboard')
        await expect(page).toHaveURL(/\/login/, { timeout: 10_000 })
    })

    test('login page redirects authenticated user to dashboard', async ({ authenticatedPage }) => {
        await authenticatedPage.goto('/login')
        // Authenticated user should be redirected away from login
        await expect(authenticatedPage).toHaveURL(/\/dashboard/, { timeout: 10_000 })
    })
})
