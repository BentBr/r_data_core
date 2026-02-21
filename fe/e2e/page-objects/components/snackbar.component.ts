import { type Page, expect } from '@playwright/test'

export class SnackbarComponent {
    constructor(private readonly page: Page) {}

    private get snackbar() {
        return this.page.locator('.v-snackbar')
    }

    async expectSuccess(message?: string): Promise<void> {
        const snackbar = this.snackbar.filter({ has: this.page.locator('.bg-success, .text-success') })
        await expect(snackbar).toBeVisible({ timeout: 10_000 })
        if (message) {
            await expect(snackbar).toContainText(message)
        }
    }

    async expectError(message?: string): Promise<void> {
        const snackbar = this.snackbar.filter({ has: this.page.locator('.bg-error, .text-error') })
        await expect(snackbar).toBeVisible({ timeout: 10_000 })
        if (message) {
            await expect(snackbar).toContainText(message)
        }
    }
}
