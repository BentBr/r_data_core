import { type Page, expect } from '@playwright/test'

export class DashboardPage {
    constructor(private readonly page: Page) {}

    // Selectors
    private get statTiles() {
        return this.page.getByTestId('dashboard-stat-tile')
    }

    private get quickActionsSection() {
        return this.page.getByTestId('dashboard-quick-actions')
    }

    private get quickActionButtons() {
        return this.quickActionsSection.locator('button')
    }

    // Actions
    async goto(): Promise<void> {
        await this.page.goto('/dashboard')
        await this.page.waitForLoadState('networkidle')
    }

    async clickQuickAction(name: string): Promise<void> {
        await this.quickActionsSection.getByRole('button', { name }).click()
    }

    // Assertions
    async expectStatsLoaded(): Promise<void> {
        // Wait for loading spinners to disappear (stats have loaded)
        await expect(this.statTiles.first()).toBeVisible({ timeout: 15_000 })
        // Verify all 4 tiles are present
        await expect(this.statTiles).toHaveCount(4)
        // Verify no progress spinners remain
        await expect(this.page.locator('.v-progress-circular')).toHaveCount(0, { timeout: 10_000 })
    }

    async expectQuickActionsVisible(): Promise<void> {
        await expect(this.quickActionsSection).toBeVisible()
        // Admin should see at least one quick action button
        const count = await this.quickActionButtons.count()
        expect(count).toBeGreaterThan(0)
    }
}
