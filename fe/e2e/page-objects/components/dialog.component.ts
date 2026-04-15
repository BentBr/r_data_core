import { type Page, expect } from '@playwright/test'

export class DialogComponent {
    constructor(private readonly page: Page) {}

    private get dialog() {
        return this.page.locator('.v-dialog--active, .v-overlay--active .v-card')
    }

    async expectOpen(): Promise<void> {
        await expect(this.dialog).toBeVisible()
    }

    async close(): Promise<void> {
        // Try the close button first, then ESC
        const closeBtn = this.dialog.getByRole('button', { name: /close|cancel/i })
        if (await closeBtn.isVisible()) {
            await closeBtn.click()
        } else {
            await this.page.keyboard.press('Escape')
        }
        await expect(this.dialog).not.toBeVisible()
    }

    async confirm(): Promise<void> {
        const confirmBtn = this.dialog.getByRole('button', { name: /confirm|ok|yes|save|submit/i })
        await confirmBtn.click()
    }
}
