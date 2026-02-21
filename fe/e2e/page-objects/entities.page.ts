import { type Page, expect } from '@playwright/test'

export class EntitiesPage {
    constructor(private readonly page: Page) {}

    // Selectors
    private get createBtn() {
        return this.page.getByTestId('entities-create-btn')
    }

    private get tree() {
        return this.page.getByTestId('entities-tree')
    }

    private get details() {
        return this.page.getByTestId('entities-details')
    }

    // Actions
    async goto(): Promise<void> {
        await this.page.goto('/entities')
        await this.page.waitForLoadState('networkidle')
    }

    async clickCreate(): Promise<void> {
        await this.createBtn.click()
    }

    async fillCreateForm(entityType: string, fields: Record<string, string>): Promise<void> {
        const dialog = this.page.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible()

        // Select entity type
        const typeSelect = dialog.getByLabel(/entity.type|type/i)
        await typeSelect.click()
        await this.page.locator('.v-overlay--active .v-list-item').getByText(entityType).click()

        // Fill fields
        for (const [label, value] of Object.entries(fields)) {
            await dialog.getByLabel(new RegExp(label, 'i')).fill(value)
        }

        await dialog.getByRole('button', { name: /create|save|submit/i }).click()
    }

    async selectInTree(name: string): Promise<void> {
        await this.tree.getByText(name).click()
        await this.page.waitForLoadState('networkidle')
    }

    async editEntity(): Promise<void> {
        await this.details.getByRole('button', { name: /edit/i }).first().click()
    }

    async deleteEntity(): Promise<void> {
        await this.details.getByRole('button', { name: /delete/i }).click()

        const dialog = this.page.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible()
        await dialog.getByRole('button', { name: /confirm|delete|yes/i }).click()
    }

    // Assertions
    async expectCreateBtnVisible(): Promise<void> {
        await expect(this.createBtn).toBeVisible()
    }

    async expectTreeVisible(): Promise<void> {
        await expect(this.tree).toBeVisible()
    }

    async expectDetailsVisible(): Promise<void> {
        await expect(this.details).toBeVisible()
    }

    async expectInTree(name: string): Promise<void> {
        await expect(this.tree.getByText(name)).toBeVisible({ timeout: 10_000 })
    }

    async expectNotInTree(name: string): Promise<void> {
        await expect(this.tree.getByText(name)).not.toBeVisible({ timeout: 10_000 })
    }
}
