import { test, expect } from '../fixtures/auth.fixture'
import { EntityDefinitionsPage } from '../page-objects/entity-definitions.page'
import { NavigationComponent } from '../page-objects/components/navigation.component'

test.describe('Entity Definitions', () => {
    test.beforeEach(async ({ authenticatedPage }) => {
        // Navigate via sidebar (SPA navigation preserves auth state)
        const nav = new NavigationComponent(authenticatedPage)
        await nav.navigateTo('/entity-definitions')
        // Wait for the tree to load - check for either tree items or empty tree rendered
        await authenticatedPage.waitForLoadState('networkidle')
    })

    test('create entity definition appears in tree', async ({ authenticatedPage }) => {
        const edPage = new EntityDefinitionsPage(authenticatedPage)
        await edPage.expectCreateBtnVisible()
        await edPage.clickCreate()

        // Use .first() because the icon picker is also a .v-card inside the dialog
        const dialog = authenticatedPage.locator('.v-dialog--active, .v-overlay--active').first()
        await expect(dialog).toBeVisible()

        // Use unique name to avoid 409 conflicts
        const suffix = Date.now()
        const entityType = `e2e_article_${suffix}`
        await dialog.getByLabel(/entity.type/i).fill(entityType)
        await dialog.getByLabel(/display.name/i).fill(`E2E Article ${suffix}`)

        // Click Create button (may need scrolling since the icon picker makes dialog tall)
        const createBtn = dialog.getByRole('button', { name: /create/i }).last()
        await createBtn.scrollIntoViewIfNeeded()
        await createBtn.click()

        // Wait for dialog to close and tree to update
        await expect(dialog).not.toBeVisible({ timeout: 10_000 })
        // Tree shows both display_name (main) and entity_type (caption)
        await edPage.expectInTree(entityType)
    })

    test('select definition shows details panel', async ({ authenticatedPage }) => {
        const edPage = new EntityDefinitionsPage(authenticatedPage)

        // Wait for tree to have the e2e_test_product from global setup
        await edPage.expectInTree('e2e_test_product')

        // Select the e2e_test_product created in global setup
        await edPage.selectInTree('e2e_test_product')
        await edPage.expectDetailsVisible()
    })

    test('edit definition metadata', async ({ authenticatedPage }) => {
        const edPage = new EntityDefinitionsPage(authenticatedPage)
        await edPage.expectInTree('e2e_test_product')
        await edPage.selectInTree('e2e_test_product')
        await edPage.expectDetailsVisible()
        await edPage.clickEditMeta()

        // Find the edit dialog by its title text
        const dialog = authenticatedPage.locator('.v-overlay--active .v-card', {
            has: authenticatedPage.getByText('Edit Entity Definition'),
        })
        await expect(dialog).toBeVisible()

        // Update display name
        const displayNameInput = dialog.getByLabel(/display.name/i)
        await displayNameInput.fill('E2E Test Product Updated')

        // Wait for API response when clicking save
        const [updateResp] = await Promise.all([
            authenticatedPage.waitForResponse(
                resp =>
                    resp.url().includes('/entity-definitions/') &&
                    resp.request().method() === 'PUT'
            ),
            (async () => {
                const saveBtn = dialog.getByRole('button', { name: /save/i })
                await saveBtn.scrollIntoViewIfNeeded()
                await saveBtn.click()
            })(),
        ])
        expect(updateResp.status()).toBeLessThan(300)

        // Wait for dialog to close
        await expect(dialog).not.toBeVisible({ timeout: 15_000 })
        await authenticatedPage.waitForLoadState('networkidle')

        // Reload page to ensure tree picks up updated data from server
        await authenticatedPage.reload()
        await authenticatedPage.waitForLoadState('networkidle')

        // Verify the updated name is shown in the tree
        await edPage.expectInTree('E2E Test Product Updated')

        // Revert the name for other tests
        await edPage.selectInTree('E2E Test Product Updated')
        await edPage.clickEditMeta()
        const revertDialog = authenticatedPage.locator('.v-overlay--active .v-card', {
            has: authenticatedPage.getByText('Edit Entity Definition'),
        })
        await expect(revertDialog).toBeVisible()
        const revertInput = revertDialog.getByLabel(/display.name/i)
        await revertInput.fill('E2E Test Product')
        const [revertResp] = await Promise.all([
            authenticatedPage.waitForResponse(
                resp =>
                    resp.url().includes('/entity-definitions/') &&
                    resp.request().method() === 'PUT'
            ),
            (async () => {
                const revertBtn = revertDialog.getByRole('button', { name: /save/i })
                await revertBtn.scrollIntoViewIfNeeded()
                await revertBtn.click()
            })(),
        ])
        expect(revertResp.status()).toBeLessThan(300)
    })

    test('add field to definition appears in fields tab', async ({ authenticatedPage }) => {
        const edPage = new EntityDefinitionsPage(authenticatedPage)
        await edPage.expectInTree('e2e_test_product')
        await edPage.selectInTree('e2e_test_product')
        await edPage.expectDetailsVisible()

        // Switch to Fields tab first
        const details = authenticatedPage.getByTestId('entity-def-details')
        await details.getByRole('tab', { name: /fields/i }).click()

        // Click add field button
        const addFieldBtn = details.getByRole('button', { name: /add.field|new.field/i })
        await addFieldBtn.click()

        // Fill field form — FieldEditor component (find dialog by its title)
        const dialog = authenticatedPage.locator('.v-overlay--active .v-card', {
            has: authenticatedPage.getByText('Add Field'),
        })
        await expect(dialog).toBeVisible()

        // Use unique field name to avoid conflicts from previous runs
        const suffix = Date.now()
        const fieldName = `e2e_field_${suffix}`

        // Select field type FIRST (required) — click the v-select to open dropdown
        await dialog.locator('[data-test="field_type"]').click()
        // Wait for dropdown overlay then select "Text" (simpler type, fewer validation fields)
        await authenticatedPage.getByRole('option', { name: 'Text', exact: true }).click()

        // Fill name and display_name
        await dialog.locator('[data-test="name"] input').fill(fieldName)
        await dialog.locator('[data-test="display_name"] input').fill(`E2E Field ${suffix}`)

        // Wait for form validation to settle
        await authenticatedPage.waitForTimeout(500)

        // Save button (text: "Add") should now be enabled
        const saveBtn = dialog.locator('[data-test="save"]')
        await expect(saveBtn).toBeEnabled({ timeout: 5_000 })
        await saveBtn.scrollIntoViewIfNeeded()
        await saveBtn.click()

        // Wait for API call to complete
        await authenticatedPage.waitForLoadState('networkidle')

        // Field should appear in the details panel (both name and display_name show)
        await expect(details.getByText(fieldName)).toBeVisible({ timeout: 10_000 })
    })

    test('delete entity definition removes from tree', async ({ authenticatedPage }) => {
        const edPage = new EntityDefinitionsPage(authenticatedPage)

        // First create a definition to delete
        await edPage.clickCreate()
        const dialog = authenticatedPage.locator('.v-dialog--active, .v-overlay--active').first()
        await expect(dialog).toBeVisible()

        const suffix = Date.now()
        const entityType = `e2e_del_${suffix}`
        await dialog.getByLabel(/entity.type/i).fill(entityType)
        await dialog.getByLabel(/display.name/i).fill(`E2E Delete ${suffix}`)

        const createBtn = dialog.getByRole('button', { name: /create/i }).last()
        await createBtn.scrollIntoViewIfNeeded()
        await createBtn.click()
        await expect(dialog).not.toBeVisible({ timeout: 10_000 })

        // Select it
        await edPage.expectInTree(entityType)
        await edPage.selectInTree(entityType)
        await edPage.expectDetailsVisible()

        // Delete it
        await edPage.deleteDefinition()

        // Verify removed from tree
        await edPage.expectNotInTree(entityType)
    })
})
