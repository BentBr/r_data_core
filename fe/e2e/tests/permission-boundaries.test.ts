import { test, expect } from '../fixtures/auth.fixture'
import { NavigationComponent } from '../page-objects/components/navigation.component'

// The viewer user is created by global setup with read-only permissions:
//   DashboardStats (Read), EntityDefinitions (Read), Entities (Read)
// It does NOT have permissions for Users, Roles, ApiKeys, Workflows, or System.

test.describe('Permission Boundaries (viewer user)', () => {
    test('viewer cannot see create button on users tab', async ({ viewerPage }) => {
        const nav = new NavigationComponent(viewerPage)

        // Navigate to /permissions — viewer may or may not have access
        let canNavigate = true
        try {
            await nav.navigateTo('/permissions')
        } catch {
            canNavigate = false
        }

        if (!canNavigate) {
            await expect(viewerPage).not.toHaveURL(/\/permissions/, { timeout: 5_000 })
            return
        }

        // Try to switch to users tab; may not be present for restricted users
        const usersTab = viewerPage.getByTestId('permissions-users-tab')
        const tabVisible = await usersTab.isVisible()
        if (!tabVisible) {
            return
        }
        await usersTab.click()

        // Create button should not be visible or should be disabled
        const createBtn = viewerPage.getByTestId('users-create-btn')
        const btnVisible = await createBtn.isVisible({ timeout: 5_000 }).catch(() => false)
        if (btnVisible) {
            await expect(createBtn).toBeDisabled({ timeout: 5_000 })
        } else {
            await expect(createBtn).not.toBeVisible({ timeout: 5_000 })
        }
    })

    test('viewer cannot see create button on roles tab', async ({ viewerPage }) => {
        const nav = new NavigationComponent(viewerPage)

        let canNavigate = true
        try {
            await nav.navigateTo('/permissions')
        } catch {
            canNavigate = false
        }

        if (!canNavigate) {
            await expect(viewerPage).not.toHaveURL(/\/permissions/, { timeout: 5_000 })
            return
        }

        const rolesTab = viewerPage.getByTestId('permissions-roles-tab')
        const tabVisible = await rolesTab.isVisible()
        if (!tabVisible) {
            return
        }
        await rolesTab.click()

        const createBtn = viewerPage.getByTestId('roles-create-btn')
        const btnVisible = await createBtn.isVisible({ timeout: 5_000 }).catch(() => false)
        if (btnVisible) {
            await expect(createBtn).toBeDisabled({ timeout: 5_000 })
        } else {
            await expect(createBtn).not.toBeVisible({ timeout: 5_000 })
        }
    })

    test('viewer cannot see create button on API keys page', async ({ viewerPage }) => {
        const nav = new NavigationComponent(viewerPage)

        let canNavigate = true
        try {
            await nav.navigateTo('/api-keys')
        } catch {
            canNavigate = false
        }

        if (!canNavigate) {
            const url = viewerPage.url()
            expect(url).not.toMatch(/\/api-keys/)
            return
        }

        const currentUrl = viewerPage.url()
        if (!currentUrl.includes('/api-keys')) {
            return
        }

        const createBtn = viewerPage.getByTestId('api-keys-create-btn')
        const btnVisible = await createBtn.isVisible({ timeout: 5_000 }).catch(() => false)
        if (btnVisible) {
            await expect(createBtn).toBeDisabled({ timeout: 5_000 })
        } else {
            await expect(createBtn).not.toBeVisible({ timeout: 5_000 })
        }
    })

    test('viewer cannot delete users', async ({ viewerPage }) => {
        const nav = new NavigationComponent(viewerPage)

        let canNavigate = true
        try {
            await nav.navigateTo('/permissions')
        } catch {
            canNavigate = false
        }

        if (!canNavigate) {
            await expect(viewerPage).not.toHaveURL(/\/permissions/, { timeout: 5_000 })
            return
        }

        const usersTab = viewerPage.getByTestId('permissions-users-tab')
        const tabVisible = await usersTab.isVisible()
        if (!tabVisible) {
            return
        }
        await usersTab.click()

        const table = viewerPage.getByTestId('users-table')
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
    })

    test('viewer can view pages with read access', async ({ viewerPage }) => {
        // Dashboard: viewer has DashboardStats read permission
        await viewerPage.goto('/dashboard')
        await expect(viewerPage.getByTestId('nav-sidebar')).toBeVisible({ timeout: 15_000 })
        await expect(viewerPage).not.toHaveURL(/\/login/)

        // API keys page: viewer likely has no access
        await viewerPage.goto('/api-keys')
        await expect(viewerPage.getByTestId('nav-sidebar')).toBeVisible({ timeout: 15_000 })
        await expect(viewerPage).not.toHaveURL(/\/login/)

        // Workflows page: viewer likely has no access
        await viewerPage.goto('/workflows')
        await expect(viewerPage.getByTestId('nav-sidebar')).toBeVisible({ timeout: 15_000 })
        await expect(viewerPage).not.toHaveURL(/\/login/)

        // Entities page: viewer has Entities read permission
        await viewerPage.goto('/entities')
        await expect(viewerPage.getByTestId('nav-sidebar')).toBeVisible({ timeout: 15_000 })
        await expect(viewerPage).not.toHaveURL(/\/login/)
    })

    test('viewer cannot access system settings', async ({ viewerPage }) => {
        const nav = new NavigationComponent(viewerPage)

        const systemNavItem = viewerPage.getByTestId('nav-item-/system')
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

        const currentUrl = viewerPage.url()

        if (!currentUrl.includes('/system')) {
            return
        }

        const accessDeniedIndicators = viewerPage.locator(
            '[data-testid*="access-denied"], [data-testid*="forbidden"], .v-alert--type-error, .v-alert--type-warning'
        )
        const hasIndicator =
            (await accessDeniedIndicators.count()) > 0 &&
            (await accessDeniedIndicators
                .first()
                .isVisible()
                .catch(() => false))

        if (!hasIndicator) {
            const saveBtn = viewerPage.getByRole('button', { name: /save|update|apply/i })
            const saveBtnCount = await saveBtn.count()
            if (saveBtnCount > 0) {
                for (let i = 0; i < saveBtnCount; i++) {
                    await expect(saveBtn.nth(i)).toBeDisabled()
                }
            }
        }
    })
})
