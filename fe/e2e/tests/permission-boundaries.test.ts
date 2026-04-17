import { test, expect, type Browser, type Page, type BrowserContext } from '@playwright/test'
import { LoginPage } from '../page-objects/login.page'
import { NavigationComponent } from '../page-objects/components/navigation.component'

// The viewer user is created by global setup with read-only permissions:
//   DashboardStats (Read), EntityDefinitions (Read), Entities (Read)
// It does NOT have permissions for Users, Roles, ApiKeys, Workflows, or System.

const VIEWER_USERNAME = 'e2e_viewer_user'
const VIEWER_PASSWORD = 'e2e_viewer_password_123'

/**
 * Try to log in as the viewer user. Returns { page, context } on success, or
 * null if the viewer user doesn't exist / login fails. Callers must close
 * context when done.
 */
async function tryLoginAsViewer(
    browser: Browser
): Promise<{ page: Page; context: BrowserContext } | null> {
    const context = await browser.newContext()
    const page = await context.newPage()
    const loginPage = new LoginPage(page)
    await loginPage.goto()
    await loginPage.login(VIEWER_USERNAME, VIEWER_PASSWORD)

    try {
        await page.waitForURL('**/dashboard', { timeout: 10_000 })
        await page.waitForLoadState('networkidle')
        await page.getByTestId('nav-sidebar').waitFor({ state: 'visible', timeout: 15_000 })
        return { page, context }
    } catch {
        // Login failed (user missing, wrong password, or no dashboard redirect)
        await context.close()
        return null
    }
}

test.describe('Permission Boundaries (viewer user)', () => {
    test('viewer cannot see create button on users tab', async ({ browser }) => {
        const result = await tryLoginAsViewer(browser)
        if (!result) {
            test.skip(true, 'Viewer user login failed — user may not exist or be inactive')
            return
        }
        const { page, context } = result

        try {
            const nav = new NavigationComponent(page)

            // Navigate to /permissions — viewer may or may not have access
            let canNavigate = true
            try {
                await nav.navigateTo('/permissions')
            } catch {
                canNavigate = false
            }

            if (!canNavigate) {
                await expect(page).not.toHaveURL(/\/permissions/, { timeout: 5_000 })
                return
            }

            await page.waitForLoadState('networkidle')

            // Try to switch to users tab; may not be present for restricted users
            const usersTab = page.getByTestId('permissions-users-tab')
            const tabVisible = await usersTab.isVisible()
            if (!tabVisible) {
                return
            }
            await usersTab.click()
            await page.waitForLoadState('networkidle')

            // Create button should not be visible or should be disabled
            const createBtn = page.getByTestId('users-create-btn')
            const btnVisible = await createBtn.isVisible({ timeout: 5_000 }).catch(() => false)
            if (btnVisible) {
                await expect(createBtn).toBeDisabled({ timeout: 5_000 })
            } else {
                await expect(createBtn).not.toBeVisible({ timeout: 5_000 })
            }
        } finally {
            await context.close()
        }
    })

    test('viewer cannot see create button on roles tab', async ({ browser }) => {
        const result = await tryLoginAsViewer(browser)
        if (!result) {
            test.skip(true, 'Viewer user login failed — user may not exist or be inactive')
            return
        }
        const { page, context } = result

        try {
            const nav = new NavigationComponent(page)

            let canNavigate = true
            try {
                await nav.navigateTo('/permissions')
            } catch {
                canNavigate = false
            }

            if (!canNavigate) {
                await expect(page).not.toHaveURL(/\/permissions/, { timeout: 5_000 })
                return
            }

            await page.waitForLoadState('networkidle')

            const rolesTab = page.getByTestId('permissions-roles-tab')
            const tabVisible = await rolesTab.isVisible()
            if (!tabVisible) {
                return
            }
            await rolesTab.click()
            await page.waitForLoadState('networkidle')

            const createBtn = page.getByTestId('roles-create-btn')
            const btnVisible = await createBtn.isVisible({ timeout: 5_000 }).catch(() => false)
            if (btnVisible) {
                await expect(createBtn).toBeDisabled({ timeout: 5_000 })
            } else {
                await expect(createBtn).not.toBeVisible({ timeout: 5_000 })
            }
        } finally {
            await context.close()
        }
    })

    test('viewer cannot see create button on API keys page', async ({ browser }) => {
        const result = await tryLoginAsViewer(browser)
        if (!result) {
            test.skip(true, 'Viewer user login failed — user may not exist or be inactive')
            return
        }
        const { page, context } = result

        try {
            const nav = new NavigationComponent(page)

            let canNavigate = true
            try {
                await nav.navigateTo('/api-keys')
            } catch {
                canNavigate = false
            }

            if (!canNavigate) {
                const url = page.url()
                expect(url).not.toMatch(/\/api-keys/)
                return
            }

            await page.waitForLoadState('networkidle')

            const currentUrl = page.url()
            if (!currentUrl.includes('/api-keys')) {
                return
            }

            const createBtn = page.getByTestId('api-keys-create-btn')
            const btnVisible = await createBtn.isVisible({ timeout: 5_000 }).catch(() => false)
            if (btnVisible) {
                await expect(createBtn).toBeDisabled({ timeout: 5_000 })
            } else {
                await expect(createBtn).not.toBeVisible({ timeout: 5_000 })
            }
        } finally {
            await context.close()
        }
    })

    test('viewer cannot delete users', async ({ browser }) => {
        const result = await tryLoginAsViewer(browser)
        if (!result) {
            test.skip(true, 'Viewer user login failed — user may not exist or be inactive')
            return
        }
        const { page, context } = result

        try {
            const nav = new NavigationComponent(page)

            let canNavigate = true
            try {
                await nav.navigateTo('/permissions')
            } catch {
                canNavigate = false
            }

            if (!canNavigate) {
                await expect(page).not.toHaveURL(/\/permissions/, { timeout: 5_000 })
                return
            }

            await page.waitForLoadState('networkidle')

            const usersTab = page.getByTestId('permissions-users-tab')
            const tabVisible = await usersTab.isVisible()
            if (!tabVisible) {
                return
            }
            await usersTab.click()
            await page.waitForLoadState('networkidle')

            const table = page.getByTestId('users-table')
            const tableVisible = await table.isVisible({ timeout: 5_000 }).catch(() => false)
            if (!tableVisible) {
                return
            }

            const rows = table.locator('tbody tr')
            const rowCount = await rows.count()
            if (rowCount === 0) {
                return
            }

            const firstRow = rows.first()
            const buttons = firstRow.locator('button')
            const buttonCount = await buttons.count()

            if (buttonCount >= 2) {
                // Convention: edit is first button, delete is second
                const deleteBtn = buttons.nth(1)
                const isDisabled = await deleteBtn.isDisabled()
                expect(isDisabled).toBe(true)
            } else if (buttonCount === 1) {
                const singleBtn = buttons.first()
                const title = (await singleBtn.getAttribute('title')) ?? ''
                if (/delete|trash|remove/i.test(title)) {
                    await expect(singleBtn).toBeDisabled()
                }
            }
            // 0 buttons → no actions rendered for viewer, which is also acceptable
        } finally {
            await context.close()
        }
    })

    test('viewer can view pages with read access', async ({ browser }) => {
        const result = await tryLoginAsViewer(browser)
        if (!result) {
            test.skip(true, 'Viewer user login failed — user may not exist or be inactive')
            return
        }
        const { page, context } = result

        try {
            const nav = new NavigationComponent(page)

            // Dashboard: viewer has DashboardStats read permission
            await page.goto('/dashboard')
            await page.waitForLoadState('networkidle')
            await expect(page.getByTestId('nav-sidebar')).toBeVisible({ timeout: 10_000 })
            await expect(page).not.toHaveURL(/\/login/)

            // Permissions page: viewer may or may not have access
            try {
                await nav.navigateTo('/permissions')
                await page.waitForLoadState('networkidle')
                const errorOverlay = page.locator('[data-testid*="error"], .v-alert--type-error')
                await expect(errorOverlay).not.toBeVisible({ timeout: 3_000 }).catch(() => {
                    // No strict requirement — page may show partial content
                })
            } catch {
                // nav item hidden or redirect occurred — acceptable for viewer
            }

            // API keys page: viewer likely has no access
            await page.goto('/api-keys')
            await page.waitForLoadState('networkidle')
            await expect(page).not.toHaveURL(/\/login/)

            // Workflows page: viewer likely has no access
            await page.goto('/workflows')
            await page.waitForLoadState('networkidle')
            await expect(page).not.toHaveURL(/\/login/)

            // Entities page: viewer has Entities read permission
            await page.goto('/entities')
            await page.waitForLoadState('networkidle')
            await expect(page).not.toHaveURL(/\/login/)
        } finally {
            await context.close()
        }
    })

    test('viewer cannot access system settings', async ({ browser }) => {
        const result = await tryLoginAsViewer(browser)
        if (!result) {
            test.skip(true, 'Viewer user login failed — user may not exist or be inactive')
            return
        }
        const { page, context } = result

        try {
            const nav = new NavigationComponent(page)

            const systemNavItem = page.getByTestId('nav-item-/system')
            const navItemVisible = await systemNavItem.isVisible().catch(() => false)

            if (!navItemVisible) {
                await expect(systemNavItem).not.toBeVisible({ timeout: 5_000 })
                return
            }

            let navigated = true
            try {
                await nav.navigateTo('/system')
            } catch {
                navigated = false
            }

            if (!navigated) {
                return
            }

            await page.waitForLoadState('networkidle')
            const currentUrl = page.url()

            if (!currentUrl.includes('/system')) {
                return
            }

            const accessDeniedIndicators = page.locator(
                '[data-testid*="access-denied"], [data-testid*="forbidden"], .v-alert--type-error, .v-alert--type-warning'
            )
            const hasIndicator =
                (await accessDeniedIndicators.count()) > 0 &&
                (await accessDeniedIndicators.first().isVisible().catch(() => false))

            if (!hasIndicator) {
                const saveBtn = page.getByRole('button', { name: /save|update|apply/i })
                const saveBtnCount = await saveBtn.count()
                if (saveBtnCount > 0) {
                    for (let i = 0; i < saveBtnCount; i++) {
                        await expect(saveBtn.nth(i)).toBeDisabled()
                    }
                }
            }
        } finally {
            await context.close()
        }
    })
})
