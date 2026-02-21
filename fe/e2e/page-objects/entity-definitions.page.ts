import { type Page, expect } from '@playwright/test'

export class EntityDefinitionsPage {
    constructor(private readonly page: Page) {}

    // Selectors
    private get createBtn() {
        return this.page.getByTestId('entity-def-create-btn')
    }

    private get tree() {
        return this.page.getByTestId('entity-def-tree')
    }

    private get details() {
        return this.page.getByTestId('entity-def-details')
    }

    // Actions
    async goto(): Promise<void> {
        await this.page.goto('/entity-definitions')
        await this.page.waitForLoadState('networkidle')
    }

    async clickCreate(): Promise<void> {
        await this.createBtn.click()
    }

    async fillCreateForm(entityType: string, displayName: string): Promise<void> {
        const dialog = this.page.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible()
        await dialog.getByLabel(/entity.type|entity_type/i).fill(entityType)
        await dialog.getByLabel(/display.name|display_name/i).fill(displayName)
        await dialog.getByRole('button', { name: /create|save|submit/i }).click()
    }

    async selectInTree(name: string): Promise<void> {
        await this.tree.getByText(name).click()
        // Wait for details to load
        await this.page.waitForLoadState('networkidle')
    }

    async clickEditMeta(): Promise<void> {
        await this.details.getByRole('button', { name: /edit/i }).first().click()
    }

    async addField(fieldName: string, displayName: string, fieldType: string): Promise<void> {
        await this.details.getByRole('button', { name: /add.field/i }).click()

        const dialog = this.page.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible()
        await dialog.getByLabel(/^name$/i).fill(fieldName)
        await dialog.getByLabel(/display.name/i).fill(displayName)

        // Select field type from dropdown
        const typeSelect = dialog.getByLabel(/field.type|type/i)
        await typeSelect.click()
        await this.page.locator('.v-overlay--active .v-list-item').getByText(fieldType).click()

        await dialog.getByRole('button', { name: /save|add|submit/i }).click()
    }

    async deleteDefinition(): Promise<void> {
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
