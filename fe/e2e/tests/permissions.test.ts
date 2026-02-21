import { test, expect } from '../fixtures/auth.fixture'
import { PermissionsPage } from '../page-objects/permissions.page'
import { NavigationComponent } from '../page-objects/components/navigation.component'

test.describe('Permissions', () => {
    test.beforeEach(async ({ authenticatedPage }) => {
        const nav = new NavigationComponent(authenticatedPage)
        await nav.navigateTo('/permissions')
    })

    test('create user appears in users table', async ({ authenticatedPage }) => {
        const permissions = new PermissionsPage(authenticatedPage)
        await permissions.switchToUsersTab()

        // Use unique name to avoid 409 conflicts from previous runs
        const suffix = Date.now()
        const username = `e2e_user_${suffix}`
        await permissions.createUser(
            username,
            `e2e_${suffix}@test.local`,
            'e2e_password_123',
            'E2E',
            'User'
        )

        // Wait for dialog to close
        const dialog = authenticatedPage.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).not.toBeVisible({ timeout: 10_000 })

        await permissions.expectUserInTable(username)
    })

    test('edit user opens edit dialog', async ({ authenticatedPage }) => {
        const permissions = new PermissionsPage(authenticatedPage)
        await permissions.switchToUsersTab()

        // Create a fresh user so it appears on page 1 (avoids pagination issues)
        const suffix = Date.now()
        const username = `e2e_edit_${suffix}`
        await permissions.createUser(
            username,
            `e2e_edit_${suffix}@test.local`,
            'e2e_password_123',
            'Edit',
            'Test'
        )
        const createDialog = authenticatedPage.locator('.v-overlay--active .v-card')
        await expect(createDialog).not.toBeVisible({ timeout: 10_000 })

        await permissions.expectUserInTable(username)
        await permissions.editUser(username)

        // Edit dialog should open
        const dialog = authenticatedPage.locator('.v-overlay--active .v-card')
        await expect(dialog).toBeVisible({ timeout: 10_000 })

        // Close without saving
        await authenticatedPage.keyboard.press('Escape')
    })

    test('create role appears in roles table', async ({ authenticatedPage }) => {
        const permissions = new PermissionsPage(authenticatedPage)
        await permissions.switchToRolesTab()

        // Use unique name to avoid 409 conflicts
        const suffix = Date.now()
        const roleName = `e2e_role_${suffix}`
        await permissions.createRole(roleName, 'E2E test role created in test')

        // Wait for dialog to close
        const dialog = authenticatedPage.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).not.toBeVisible({ timeout: 10_000 })

        await permissions.expectRoleInTable(roleName)
    })

    test('edit role opens edit dialog', async ({ authenticatedPage }) => {
        const permissions = new PermissionsPage(authenticatedPage)
        await permissions.switchToRolesTab()

        // Create a fresh role so it appears on page 1 (avoids pagination issues)
        const suffix = Date.now()
        const roleName = `e2e_editrole_${suffix}`
        await permissions.createRole(roleName, 'E2E edit test role')
        const createDialog = authenticatedPage.locator('.v-overlay--active .v-card')
        await expect(createDialog).not.toBeVisible({ timeout: 10_000 })

        await permissions.expectRoleInTable(roleName)
        await permissions.editRole(roleName)

        // Edit dialog should open
        const dialog = authenticatedPage.locator('.v-overlay--active .v-card')
        await expect(dialog).toBeVisible({ timeout: 10_000 })

        // Close without saving
        await authenticatedPage.keyboard.press('Escape')
    })

    test('delete user sets inactive status', async ({ authenticatedPage }) => {
        const permissions = new PermissionsPage(authenticatedPage)
        await permissions.switchToUsersTab()

        // Create a user to delete with unique name
        const suffix = Date.now()
        const username = `e2e_del_${suffix}`
        await permissions.createUser(
            username,
            `e2e_del_${suffix}@test.local`,
            'e2e_password_123',
            'Delete',
            'Me'
        )
        const createDialog = authenticatedPage.locator('.v-overlay--active .v-card')
        await expect(createDialog).not.toBeVisible({ timeout: 10_000 })

        await permissions.expectUserInTable(username)

        // Verify user starts as Active
        const table = authenticatedPage.getByTestId('users-table')
        const row = table.locator('tr', {
            has: authenticatedPage.getByText(username, { exact: true }),
        })
        await expect(row.getByText('Active')).toBeVisible()

        // Delete the user (soft-delete sets inactive)
        await permissions.deleteUser(username)

        // Verify user is now Inactive (soft-delete changes status, doesn't remove)
        await expect(row.getByText('Inactive')).toBeVisible({ timeout: 10_000 })
    })
})
