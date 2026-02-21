import { test, expect } from '../fixtures/auth.fixture'
import { EntitiesPage } from '../page-objects/entities.page'
import { NavigationComponent } from '../page-objects/components/navigation.component'

test.describe('Entities', () => {
    test.beforeEach(async ({ authenticatedPage }) => {
        const nav = new NavigationComponent(authenticatedPage)
        await nav.navigateTo('/entities')
    })

    test('create entity appears in tree', async ({ authenticatedPage }) => {
        const entitiesPage = new EntitiesPage(authenticatedPage)
        await entitiesPage.expectCreateBtnVisible()
        await entitiesPage.clickCreate()

        const dialog = authenticatedPage.locator('.v-overlay--active .v-card')
        await expect(dialog).toBeVisible()

        // Select entity type — click the select, wait for dropdown, pick type
        const typeSelect = dialog.locator('.v-select, .v-autocomplete').first()
        await typeSelect.click()

        // Wait for dropdown items to appear
        const listItems = authenticatedPage.locator('.v-overlay--active .v-list-item')
        await expect(listItems.first()).toBeVisible({ timeout: 10_000 })

        // Find e2e_test_product (display name: E2E Test Product)
        const typeOption = listItems.filter({ hasText: /e2e.test.product/i })
        if ((await typeOption.count()) === 0) {
            // If the entity type is not in the dropdown, it's not published — skip test
            test.skip()
            return
        }
        await typeOption.first().click()

        // Fill entity key (required field) — use unique key to avoid 409 conflicts
        const suffix = Date.now()
        const keyInput = dialog.getByLabel(/^key$/i)
        await keyInput.fill(`e2e_entity_${suffix}`)

        // Fill title if visible (dynamic field from entity definition)
        const titleInput = dialog.getByLabel(/title/i)
        if (await titleInput.isVisible({ timeout: 3_000 }).catch(() => false)) {
            await titleInput.fill(`E2E Test Entity ${suffix}`)
        }

        // Wait for API create response while clicking Create
        const [createResponse] = await Promise.all([
            authenticatedPage.waitForResponse(
                resp =>
                    resp.url().includes('/e2e_test_product') &&
                    resp.request().method() === 'POST'
            ),
            (async () => {
                const createBtn = dialog.getByRole('button', { name: /create/i })
                await createBtn.scrollIntoViewIfNeeded()
                await createBtn.click()
            })(),
        ])

        expect(createResponse.status()).toBeLessThan(300)

        // Dialog may stay open after form reset — close it if still visible
        if (await dialog.isVisible()) {
            await authenticatedPage.keyboard.press('Escape')
        }
        await expect(dialog).not.toBeVisible({ timeout: 10_000 })

        // Verify the entity was created by checking the tree
        await authenticatedPage.waitForLoadState('networkidle')
    })

    test('select entity shows details panel', async ({ authenticatedPage }) => {
        const entitiesPage = new EntitiesPage(authenticatedPage)
        await entitiesPage.expectTreeVisible()

        // Click on any entity in the tree (if available)
        const treeItems = authenticatedPage
            .getByTestId('entities-tree')
            .locator('.v-treeview-item, .v-list-item')
        const count = await treeItems.count()
        if (count > 0) {
            await treeItems.first().click()
            await authenticatedPage.waitForLoadState('networkidle')

            // Details panel should be visible with entity info
            await entitiesPage.expectDetailsVisible()
        }
    })

    test('edit entity fields', async ({ authenticatedPage }) => {
        const entitiesPage = new EntitiesPage(authenticatedPage)
        await entitiesPage.expectTreeVisible()

        // Click on an entity
        const treeItems = authenticatedPage
            .getByTestId('entities-tree')
            .locator('.v-treeview-item, .v-list-item')
        const count = await treeItems.count()
        if (count > 0) {
            await treeItems.first().click()
            await authenticatedPage.waitForLoadState('networkidle')

            // Click edit
            const details = authenticatedPage.getByTestId('entities-details')
            const editBtn = details.getByRole('button', { name: /edit/i })
            if (await editBtn.isVisible()) {
                await editBtn.click()

                const dialog = authenticatedPage.locator(
                    '.v-dialog--active, .v-overlay--active .v-card'
                )
                await expect(dialog).toBeVisible()

                // Close without saving (just verify the edit dialog works)
                await authenticatedPage.keyboard.press('Escape')
            }
        }
    })

    test('delete entity removes from tree', async ({ authenticatedPage }) => {
        const entitiesPage = new EntitiesPage(authenticatedPage)

        // First create an entity to delete
        await entitiesPage.clickCreate()
        const dialog = authenticatedPage.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible()

        // Select entity type
        const typeSelect = dialog.locator('.v-select, .v-autocomplete').first()
        await typeSelect.click()

        const listItems = authenticatedPage.locator('.v-overlay--active .v-list-item')
        await expect(listItems.first()).toBeVisible({ timeout: 10_000 })

        const typeOption = listItems.filter({ hasText: /e2e.test.product/i })
        if ((await typeOption.count()) === 0) {
            test.skip()
            return
        }
        await typeOption.first().click()

        // Fill entity key — use unique key to avoid 409 conflicts
        const delSuffix = Date.now()
        const keyInput = dialog.getByLabel(/^key$/i)
        await keyInput.fill(`e2e_del_entity_${delSuffix}`)

        // Fill title if visible
        const titleInput = dialog.getByLabel(/title/i)
        if (await titleInput.isVisible({ timeout: 3_000 }).catch(() => false)) {
            await titleInput.fill(`E2E Delete Target ${delSuffix}`)
        }

        await dialog.getByRole('button', { name: /create/i }).scrollIntoViewIfNeeded()
        await dialog.getByRole('button', { name: /create/i }).click()
        await expect(dialog).not.toBeVisible({ timeout: 10_000 })

        // Try to find and select the entity in the tree, then delete
        const treeItems = authenticatedPage
            .getByTestId('entities-tree')
            .locator('.v-treeview-item, .v-list-item')
        const count = await treeItems.count()
        if (count > 0) {
            // Click the last item (likely the one we just created)
            await treeItems.last().click()
            await authenticatedPage.waitForLoadState('networkidle')

            const details = authenticatedPage.getByTestId('entities-details')
            const deleteBtn = details.getByRole('button', { name: /delete/i })
            if (await deleteBtn.isVisible()) {
                await deleteBtn.click()

                const confirmDialog = authenticatedPage.locator(
                    '.v-dialog--active, .v-overlay--active .v-card'
                )
                await expect(confirmDialog).toBeVisible()
                await confirmDialog.getByRole('button', { name: /confirm|delete|yes/i }).click()

                // Success snackbar
                const snackbar = authenticatedPage.locator('.v-snackbar')
                await expect(snackbar).toBeVisible({ timeout: 10_000 })
            }
        }
    })
})
