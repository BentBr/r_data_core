import { test, expect } from '../fixtures/auth.fixture'
import { WorkflowsPage } from '../page-objects/workflows.page'
import { NavigationComponent } from '../page-objects/components/navigation.component'

test.describe('Workflows', () => {
    test.beforeEach(async ({ authenticatedPage }) => {
        const nav = new NavigationComponent(authenticatedPage)
        await nav.navigateTo('/workflows')
    })

    test('workflow from setup appears in table', async ({ authenticatedPage }) => {
        const workflows = new WorkflowsPage(authenticatedPage)
        await workflows.expectTableVisible()
        await workflows.expectInTable('e2e_test_workflow')
    })

    test('edit workflow opens edit dialog', async ({ authenticatedPage }) => {
        const workflows = new WorkflowsPage(authenticatedPage)
        await workflows.expectTableVisible()
        await workflows.editWorkflow('e2e_test_workflow')

        // Edit dialog should open
        const dialog = authenticatedPage.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible({ timeout: 10_000 })

        // Close without saving
        await authenticatedPage.keyboard.press('Escape')
    })

    test('history tab shows runs table', async ({ authenticatedPage }) => {
        const workflows = new WorkflowsPage(authenticatedPage)
        await workflows.expectHistoryTabVisible()
        await workflows.switchToHistoryTab()

        // The history tab should show content
        const historyContent = authenticatedPage.locator('.v-window-item--active')
        await expect(historyContent).toBeVisible({ timeout: 10_000 })

        // Select "All" workflows to see runs (if any)
        const workflowSelect = historyContent.locator('.v-select').first()
        if (await workflowSelect.isVisible()) {
            await workflowSelect.click()
            const allOption = authenticatedPage.locator('.v-overlay--active .v-list-item').first()
            await allOption.click()
        }
    })

    test('delete workflow removes from table', async ({ authenticatedPage }) => {
        const workflows = new WorkflowsPage(authenticatedPage)
        await workflows.expectTableVisible()

        // Delete the e2e_test_workflow created in global setup
        await workflows.deleteWorkflow('e2e_test_workflow')

        // Verify removed
        await workflows.expectNotInTable('e2e_test_workflow')
    })
})
