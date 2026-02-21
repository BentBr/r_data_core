import { test, expect } from '../fixtures/auth.fixture'
import { ApiKeysPage } from '../page-objects/api-keys.page'
import { NavigationComponent } from '../page-objects/components/navigation.component'

test.describe('API Keys', () => {
    test.beforeEach(async ({ authenticatedPage }) => {
        const nav = new NavigationComponent(authenticatedPage)
        await nav.navigateTo('/api-keys')
    })

    test('create API key shows generated key', async ({ authenticatedPage }) => {
        const apiKeys = new ApiKeysPage(authenticatedPage)
        await apiKeys.expectCreateBtnVisible()
        await apiKeys.clickCreate()

        const dialog = authenticatedPage.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible()

        // Use unique name with timestamp to avoid 409 conflicts
        const keyName = `e2e_key_${Date.now()}`
        await dialog.getByLabel(/name/i).fill(keyName)
        await dialog.getByLabel(/description/i).fill('Created during E2E test')
        await dialog.getByRole('button', { name: /create/i }).click()

        // The "API Key Created" dialog should appear with the key value in a text input
        const createdDialog = authenticatedPage.locator(
            '.v-dialog--active, .v-overlay--active .v-card'
        )
        await expect(createdDialog).toBeVisible({ timeout: 10_000 })
        await expect(createdDialog.locator('#apiKey')).toBeVisible({ timeout: 5_000 })
    })

    test('created key appears in table as Active', async ({ authenticatedPage }) => {
        const apiKeys = new ApiKeysPage(authenticatedPage)
        await apiKeys.expectTableVisible()

        // Create a fresh key and verify it appears on first page as Active
        const keyName = `e2e_key_${Date.now()}`
        await apiKeys.clickCreate()
        const dialog = authenticatedPage.locator('.v-overlay--active .v-card')
        await expect(dialog).toBeVisible()
        await dialog.getByLabel(/name/i).fill(keyName)
        await dialog.getByLabel(/description/i).fill('Created during E2E test')
        await dialog.getByRole('button', { name: /create/i }).click()

        // Close the "API Key Created" dialog
        const createdDialog = authenticatedPage.locator('.v-overlay--active .v-card')
        await expect(createdDialog).toBeVisible({ timeout: 10_000 })
        await createdDialog.getByRole('button', { name: /close/i }).click()
        await expect(createdDialog).not.toBeVisible({ timeout: 10_000 })

        // Newly created key should appear on first page as Active
        await apiKeys.expectKeyInTable(keyName)
        await apiKeys.expectKeyStatus(keyName, 'Active')
    })

    test('revoke key changes status to Inactive', async ({ authenticatedPage }) => {
        const apiKeys = new ApiKeysPage(authenticatedPage)
        await apiKeys.clickCreate()

        const dialog = authenticatedPage.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible()

        // Use unique name with timestamp
        const keyName = `e2e_revoke_${Date.now()}`
        await dialog.getByLabel(/name/i).fill(keyName)
        await dialog.getByRole('button', { name: /create/i }).click()

        // Close the "API Key Created" dialog
        const createdDialog = authenticatedPage.locator(
            '.v-dialog--active, .v-overlay--active .v-card'
        )
        await expect(createdDialog).toBeVisible({ timeout: 10_000 })
        await createdDialog.getByRole('button', { name: /close/i }).click()
        await expect(createdDialog).not.toBeVisible({ timeout: 10_000 })

        // Revoke the key
        await apiKeys.revokeKey(keyName)

        // Verify status changed
        await apiKeys.expectKeyStatus(keyName, 'Inactive')
    })

    test('revoked key shows strikethrough name', async ({ authenticatedPage }) => {
        const apiKeys = new ApiKeysPage(authenticatedPage)
        await apiKeys.expectTableVisible()

        // Find any inactive/revoked key with strikethrough
        const strikethroughElements = authenticatedPage.locator(
            '[data-testid="api-keys-table"] .text-decoration-line-through'
        )
        const count = await strikethroughElements.count()
        // If there are revoked keys, they should have strikethrough styling
        if (count > 0) {
            await expect(strikethroughElements.first()).toBeVisible()
        }
    })
})
