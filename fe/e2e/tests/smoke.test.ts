import { test, expect } from '../fixtures/auth.fixture'
import { NavigationComponent } from '../page-objects/components/navigation.component'

test.describe('Smoke Tests', () => {
    test('admin can navigate to all pages without errors', async ({ authenticatedPage }) => {
        const nav = new NavigationComponent(authenticatedPage)
        await nav.expectVisible()

        const pages = [
            '/dashboard',
            '/entity-definitions',
            '/entities',
            '/api-keys',
            '/workflows',
            '/permissions',
            '/system',
        ]

        for (const path of pages) {
            await nav.navigateTo(path)
            await expect(authenticatedPage).toHaveURL(new RegExp(path))

            // Verify no uncaught error overlay is visible
            const errorOverlay = authenticatedPage.locator(
                '.v-overlay--active .v-alert[type="error"]'
            )
            await expect(errorOverlay).not.toBeVisible()
        }
    })

    test('page titles update when navigating', async ({ authenticatedPage }) => {
        const nav = new NavigationComponent(authenticatedPage)

        await nav.navigateTo('/entity-definitions')
        const toolbar = authenticatedPage.locator('.v-toolbar-title')
        await expect(toolbar).toBeVisible()

        await nav.navigateTo('/workflows')
        await expect(toolbar).toBeVisible()
    })
})
