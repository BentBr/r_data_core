import { type Page, expect } from '@playwright/test'

export class WorkflowsPage {
    constructor(private readonly page: Page) {}

    // Selectors
    private get createBtn() {
        return this.page.getByTestId('workflows-create-btn')
    }

    private get table() {
        return this.page.getByTestId('workflows-table')
    }

    private get historyTab() {
        return this.page.getByTestId('workflows-history-tab')
    }

    // Actions
    async goto(): Promise<void> {
        await this.page.goto('/workflows')
        await this.page.waitForLoadState('networkidle')
    }

    async clickCreate(): Promise<void> {
        await this.createBtn.click()
    }

    async editWorkflow(name: string): Promise<void> {
        const row = this.table.locator('tr', { has: this.page.getByText(name, { exact: true }) })
        // Edit is the 3rd action button (play, history, pencil, trash)
        const buttons = row.locator('button')
        // Find the pencil/edit button by its position or title
        const editBtn = row.getByTitle(/edit/i).or(buttons.nth(2))
        await editBtn.first().click()
    }

    async deleteWorkflow(name: string): Promise<void> {
        const row = this.table.locator('tr', { has: this.page.getByText(name, { exact: true }) })
        // Delete is the last action button (trash icon)
        const deleteBtn = row.getByTitle(/delete/i).or(row.locator('button').last())
        await deleteBtn.first().click()

        const dialog = this.page.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible()
        await dialog.getByRole('button', { name: /confirm|delete|yes/i }).click()
        await expect(dialog).not.toBeVisible({ timeout: 10_000 })
    }

    async switchToHistoryTab(): Promise<void> {
        await this.historyTab.click()
    }

    // Assertions
    async expectCreateBtnVisible(): Promise<void> {
        await expect(this.createBtn).toBeVisible()
    }

    async expectTableVisible(): Promise<void> {
        await expect(this.table).toBeVisible()
    }

    async expectInTable(name: string): Promise<void> {
        await expect(this.table.getByText(name)).toBeVisible({ timeout: 10_000 })
    }

    async expectNotInTable(name: string): Promise<void> {
        await expect(this.table.getByText(name)).not.toBeVisible({ timeout: 10_000 })
    }

    async expectHistoryTabVisible(): Promise<void> {
        await expect(this.historyTab).toBeVisible()
    }

    async expectHistoryTableVisible(): Promise<void> {
        // After switching to history tab, a runs table should appear
        const runsTable = this.page.locator(
            '.v-window-item--active .v-data-table, .v-window-item--active table'
        )
        await expect(runsTable).toBeVisible({ timeout: 10_000 })
    }
}
