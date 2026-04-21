import { test, expect } from '../fixtures/auth.fixture'
import { SystemPage } from '../page-objects/system.page'
import { NavigationComponent } from '../page-objects/components/navigation.component'
import { SnackbarComponent } from '../page-objects/components/snackbar.component'

test.describe('System Tabs Navigation', () => {
    test.beforeEach(async ({ authenticatedPage }) => {
        const nav = new NavigationComponent(authenticatedPage)
        await nav.navigateTo('/system')
    })

    test('system page has three tabs', async ({ authenticatedPage }) => {
        await expect(authenticatedPage.getByRole('tab', { name: /settings/i })).toBeVisible()
        await expect(authenticatedPage.getByRole('tab', { name: /email templates/i })).toBeVisible()
        await expect(authenticatedPage.getByRole('tab', { name: /logs/i })).toBeVisible()
    })

    test('email templates tab shows template list', async ({ authenticatedPage }) => {
        const system = new SystemPage(authenticatedPage)
        await system.clickTab('email-templates')
        await system.expectEmailTemplatesTab()
        // Password Reset system template should exist from seed
        await system.expectSystemTemplateExists('Password Reset')
    })

    test('system logs tab shows log viewer', async ({ authenticatedPage }) => {
        const system = new SystemPage(authenticatedPage)
        await system.clickTab('logs')
        await system.expectSystemLogsTab()
    })
})

test.describe('System Settings', () => {
    test.beforeEach(async ({ authenticatedPage }) => {
        const nav = new NavigationComponent(authenticatedPage)
        await nav.navigateTo('/system')
    })

    test('license card loads with data', async ({ authenticatedPage }) => {
        const system = new SystemPage(authenticatedPage)
        await system.expectLicenseLoaded()

        // License card should show a state chip
        const licenseCard = authenticatedPage.getByTestId('system-license-card')
        await expect(licenseCard.locator('.v-chip')).toBeVisible({ timeout: 10_000 })
    })

    test('update entity versioning settings and save', async ({ authenticatedPage }) => {
        const system = new SystemPage(authenticatedPage)
        const snackbar = new SnackbarComponent(authenticatedPage)

        await system.expectVersioningSettings()
        await system.updateVersioning('50')
        await system.saveVersioning()

        await snackbar.expectSuccess()
    })

    test('update workflow run logs settings and save', async ({ authenticatedPage }) => {
        const system = new SystemPage(authenticatedPage)
        const snackbar = new SnackbarComponent(authenticatedPage)

        await system.expectRunLogsSettings()
        await system.updateRunLogs('100')
        await system.saveRunLogs()

        await snackbar.expectSuccess()
    })
})
