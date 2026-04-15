import { type Page, expect } from '@playwright/test'

export class ApiKeysPage {
    constructor(private readonly page: Page) {}

    // Selectors
    private get createBtn() {
        return this.page.getByTestId('api-keys-create-btn')
    }

    private get table() {
        return this.page.getByTestId('api-keys-table')
    }

    // Actions
    async goto(): Promise<void> {
        await this.page.goto('/api-keys')
        await this.page.waitForLoadState('networkidle')
    }

    async clickCreate(): Promise<void> {
        await this.createBtn.click()
    }

    async fillCreateForm(name: string, description?: string): Promise<void> {
        const dialog = this.page.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible()
        await dialog.getByLabel(/name/i).fill(name)
        if (description) {
            await dialog.getByLabel(/description/i).fill(description)
        }
        await dialog.getByRole('button', { name: /create|save|submit/i }).click()
    }

    async expectKeyInTable(name: string): Promise<void> {
        await expect(this.table.getByText(name)).toBeVisible({ timeout: 10_000 })
    }

    async revokeKey(name: string): Promise<void> {
        // Find the row with the key name and click the last button (trash/delete icon)
        const row = this.table.locator('tr', { has: this.page.getByText(name, { exact: true }) })
        await row.locator('button').last().click()

        // Confirm revocation in dialog
        const dialog = this.page.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible()
        await dialog.getByRole('button', { name: /revoke|confirm|delete|yes/i }).click()
    }

    // Assertions
    async expectCreateBtnVisible(): Promise<void> {
        await expect(this.createBtn).toBeVisible()
    }

    async expectTableVisible(): Promise<void> {
        await expect(this.table).toBeVisible()
    }

    async expectKeyStatus(name: string, status: 'Active' | 'Inactive'): Promise<void> {
        const row = this.table.locator('tr', { has: this.page.getByText(name, { exact: true }) })
        await expect(row.getByText(status)).toBeVisible({ timeout: 10_000 })
    }

    async expectKeyStrikethrough(name: string): Promise<void> {
        const strikethrough = this.table.locator('.text-decoration-line-through', {
            hasText: name,
        })
        await expect(strikethrough).toBeVisible({ timeout: 10_000 })
    }
}
