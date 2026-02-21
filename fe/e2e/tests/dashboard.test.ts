import { test, expect } from '../fixtures/auth.fixture'
import { DashboardPage } from '../page-objects/dashboard.page'

test.describe('Dashboard', () => {
    test('stats tiles render with data', async ({ authenticatedPage }) => {
        const dashboard = new DashboardPage(authenticatedPage)
        await dashboard.expectStatsLoaded()
    })

    test('quick actions visible for admin', async ({ authenticatedPage }) => {
        const dashboard = new DashboardPage(authenticatedPage)
        await dashboard.expectQuickActionsVisible()
    })

    test('quick action navigates correctly', async ({ authenticatedPage }) => {
        const dashboard = new DashboardPage(authenticatedPage)
        await dashboard.expectQuickActionsVisible()

        // Click the first quick action button
        const quickActions = authenticatedPage.getByTestId('dashboard-quick-actions')
        const firstButton = quickActions.locator('button').first()
        await firstButton.click()

        // Should navigate away from dashboard (URL includes ?create=true)
        await authenticatedPage.waitForURL(/create=true/, { timeout: 10_000 })
        await expect(authenticatedPage).toHaveURL(/create=true/)
    })
})
