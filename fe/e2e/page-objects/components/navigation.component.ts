import { type Page, expect } from '@playwright/test'

export class NavigationComponent {
    constructor(private readonly page: Page) {}

    // Selectors
    private get sidebar() {
        return this.page.getByTestId('nav-sidebar')
    }

    private navItem(path: string) {
        return this.sidebar.locator(`[data-testid="nav-item-${path}"]`)
    }

    // Actions
    async navigateTo(path: string): Promise<void> {
        await this.navItem(path).click()
        // Vue router may briefly redirect through login during auth check; wait for final URL
        await this.page.waitForURL(`**${path}**`, { timeout: 15_000 })
    }

    // Assertions
    async expectVisible(): Promise<void> {
        await expect(this.sidebar).toBeVisible()
    }

    async expectItemVisible(path: string): Promise<void> {
        await expect(this.navItem(path)).toBeVisible()
    }

    async expectItemNotVisible(path: string): Promise<void> {
        await expect(this.navItem(path)).not.toBeVisible()
    }
}
