import { type Page, expect } from '@playwright/test'

export class PermissionsPage {
    constructor(private readonly page: Page) {}

    // Selectors
    private get usersTab() {
        return this.page.getByTestId('permissions-users-tab')
    }

    private get rolesTab() {
        return this.page.getByTestId('permissions-roles-tab')
    }

    private get usersCreateBtn() {
        return this.page.getByTestId('users-create-btn')
    }

    private get rolesCreateBtn() {
        return this.page.getByTestId('roles-create-btn')
    }

    private get usersTable() {
        return this.page.getByTestId('users-table')
    }

    private get rolesTable() {
        return this.page.getByTestId('roles-table')
    }

    // Actions
    async goto(): Promise<void> {
        await this.page.goto('/permissions')
        await this.page.waitForLoadState('networkidle')
    }

    async switchToUsersTab(): Promise<void> {
        await this.usersTab.click()
    }

    async switchToRolesTab(): Promise<void> {
        await this.rolesTab.click()
    }

    async createUser(
        username: string,
        email: string,
        password: string,
        firstName?: string,
        lastName?: string
    ): Promise<void> {
        await this.usersCreateBtn.click()

        const dialog = this.page.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible()
        await dialog.getByLabel(/username/i).fill(username)
        await dialog.getByLabel(/email/i).fill(email)
        await dialog
            .getByLabel(/password/i)
            .first()
            .fill(password)

        if (firstName) {
            await dialog.getByLabel(/first.name/i).fill(firstName)
        }
        if (lastName) {
            await dialog.getByLabel(/last.name/i).fill(lastName)
        }

        await dialog.getByRole('button', { name: /create|save|submit/i }).click()
    }

    async editUser(username: string): Promise<void> {
        const row = this.usersTable.locator('tr', {
            has: this.page.getByText(username, { exact: true }),
        })
        // Edit is the first action button (pencil icon), delete is the second (trash icon)
        const editBtn = row.getByTitle(/edit/i).or(row.locator('button').first())
        await editBtn.first().click()
    }

    async deleteUser(username: string): Promise<void> {
        const row = this.usersTable.locator('tr', {
            has: this.page.getByText(username, { exact: true }),
        })
        // Delete is the second button in the row (trash icon)
        await row.locator('button').nth(1).click()

        const dialog = this.page.locator('.v-overlay--active .v-card')
        await expect(dialog).toBeVisible()

        // Wait for delete API response while clicking confirm
        const deleteResponse = this.page.waitForResponse(
            resp => resp.url().includes('/users/') && resp.request().method() === 'DELETE'
        )
        await dialog.getByRole('button', { name: /confirm|delete|yes/i }).click()
        await deleteResponse

        await expect(dialog).not.toBeVisible({ timeout: 10_000 })
        await this.page.waitForLoadState('networkidle')
    }

    async createRole(name: string, description: string): Promise<void> {
        await this.rolesCreateBtn.click()

        const dialog = this.page.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible()
        await dialog.getByLabel(/name/i).fill(name)
        await dialog.getByLabel(/description/i).fill(description)
        await dialog.getByRole('button', { name: /create|save|submit/i }).click()
    }

    async editRole(name: string): Promise<void> {
        const row = this.rolesTable.locator('tr', {
            has: this.page.getByText(name, { exact: true }),
        })
        await row.locator('button').first().click()
    }

    // Assertions
    async expectUsersTabVisible(): Promise<void> {
        await expect(this.usersTab).toBeVisible()
    }

    async expectRolesTabVisible(): Promise<void> {
        await expect(this.rolesTab).toBeVisible()
    }

    async expectUserInTable(username: string): Promise<void> {
        await expect(this.usersTable.getByText(username, { exact: true })).toBeVisible({
            timeout: 10_000,
        })
    }

    async expectUserNotInTable(username: string): Promise<void> {
        await expect(this.usersTable.getByText(username, { exact: true })).not.toBeVisible({
            timeout: 10_000,
        })
    }

    async expectRoleInTable(name: string): Promise<void> {
        await expect(this.rolesTable.getByText(name)).toBeVisible({ timeout: 10_000 })
    }

    async expectRoleNotInTable(name: string): Promise<void> {
        await expect(this.rolesTable.getByText(name)).not.toBeVisible({ timeout: 10_000 })
    }
}
