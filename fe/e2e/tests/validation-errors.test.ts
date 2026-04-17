import { test, expect } from '../fixtures/auth.fixture'
import { type Page, type Locator } from '@playwright/test'
import { NavigationComponent } from '../page-objects/components/navigation.component'
import { PermissionsPage } from '../page-objects/permissions.page'
import { ApiKeysPage } from '../page-objects/api-keys.page'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Open the user-create dialog and wait for it to be visible. */
async function openUserCreateDialog(page: Page): Promise<Locator> {
    await page.getByTestId('users-create-btn').click()
    const dialog = page.locator('.v-dialog--active, .v-overlay--active .v-card')
    await expect(dialog).toBeVisible({ timeout: 10_000 })
    return dialog
}

/** Fill all user-create fields and click the submit button. */
async function fillAndSubmitUserForm(
    dialog: Locator,
    opts: {
        username: string
        email: string
        password: string
        firstName?: string
        lastName?: string
    }
): Promise<void> {
    await dialog.getByLabel(/username/i).fill(opts.username)
    await dialog.getByLabel(/email/i).fill(opts.email)
    await dialog
        .getByLabel(/password/i)
        .first()
        .fill(opts.password)
    if (opts.firstName) await dialog.getByLabel(/first.name/i).fill(opts.firstName)
    if (opts.lastName) await dialog.getByLabel(/last.name/i).fill(opts.lastName)
    await dialog.getByRole('button', { name: /create|save|submit/i }).click()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test.describe('Validation errors — user creation', () => {
    test.beforeEach(async ({ authenticatedPage }) => {
        const nav = new NavigationComponent(authenticatedPage)
        await nav.navigateTo('/permissions')
        await authenticatedPage.getByTestId('permissions-users-tab').click()
    })

    test('short username shows validation error', async ({ authenticatedPage }) => {
        const dialog = await openUserCreateDialog(authenticatedPage)

        // Fill with short username and tab through other fields to trigger validation
        await dialog.getByLabel(/username/i).fill('ab')
        await dialog.getByLabel(/email/i).fill(`valid_${Date.now()}@test.local`)
        await dialog
            .getByLabel(/password/i)
            .first()
            .fill('validpassword123')
        await dialog.getByLabel(/first.name/i).fill('Test')
        await dialog.getByLabel(/last.name/i).fill('User')

        // Create button should be disabled due to client-side validation error
        const createBtn = dialog.getByRole('button', { name: /create/i })
        await expect(createBtn).toBeDisabled({ timeout: 5_000 })

        // Vuetify renders validation errors in [role="alert"] elements
        const alertMsg = dialog.locator('[role="alert"]')
        await expect(alertMsg.first()).toBeVisible({ timeout: 5_000 })

        // Dialog must still be open
        await expect(dialog).toBeVisible()
    })

    test('short password shows validation error', async ({ authenticatedPage }) => {
        const dialog = await openUserCreateDialog(authenticatedPage)

        // Fill with short password and tab through other fields to trigger validation
        await dialog.getByLabel(/username/i).fill(`e2e_val_${Date.now()}`)
        await dialog.getByLabel(/email/i).fill(`valid2_${Date.now()}@test.local`)
        await dialog
            .getByLabel(/password/i)
            .first()
            .fill('abc')
        await dialog.getByLabel(/first.name/i).fill('Test')
        await dialog.getByLabel(/last.name/i).fill('User')

        // Create button should be disabled due to client-side validation error
        const createBtn = dialog.getByRole('button', { name: /create/i })
        await expect(createBtn).toBeDisabled({ timeout: 5_000 })

        // Vuetify renders validation errors in [role="alert"] elements
        const alertMsg = dialog.locator('[role="alert"]')
        await expect(alertMsg.first()).toBeVisible({ timeout: 5_000 })

        await expect(dialog).toBeVisible()
    })

    test('invalid email shows validation error', async ({ authenticatedPage }) => {
        const dialog = await openUserCreateDialog(authenticatedPage)

        // Fill with invalid email and tab through other fields to trigger validation
        await dialog.getByLabel(/username/i).fill(`e2e_val_${Date.now()}`)
        await dialog.getByLabel(/email/i).fill('not-an-email')
        await dialog
            .getByLabel(/password/i)
            .first()
            .fill('validpassword123')
        await dialog.getByLabel(/first.name/i).fill('Test')
        await dialog.getByLabel(/last.name/i).fill('User')

        // Create button should be disabled due to client-side validation error
        const createBtn = dialog.getByRole('button', { name: /create/i })
        await expect(createBtn).toBeDisabled({ timeout: 5_000 })

        // Vuetify renders validation errors in [role="alert"] elements
        const alertMsg = dialog.locator('[role="alert"]')
        await expect(alertMsg.first()).toBeVisible({ timeout: 5_000 })

        await expect(dialog).toBeVisible()
    })

    test.describe.serial('duplicate username shows conflict error', () => {
        test('create first user, then second with same username shows error', async ({
            authenticatedPage,
        }) => {
            const permissions = new PermissionsPage(authenticatedPage)
            const suffix = Date.now()
            const username = `e2e_dup_${suffix}`
            const email1 = `e2e_dup1_${suffix}@test.local`
            const email2 = `e2e_dup2_${suffix}@test.local`

            // --- Create first user ---
            await permissions.createUser(username, email1, 'validpassword123', 'Dup', 'One')
            const firstDialog = authenticatedPage.locator(
                '.v-dialog--active, .v-overlay--active .v-card'
            )
            await expect(firstDialog).not.toBeVisible({ timeout: 10_000 })

            await permissions.expectUserInTable(username)

            // --- Attempt to create second user with same username ---
            const dialog = await openUserCreateDialog(authenticatedPage)

            await fillAndSubmitUserForm(dialog, {
                username, // duplicate
                email: email2,
                password: 'validpassword123',
                firstName: 'Dup',
                lastName: 'Two',
            })

            // Expect a 409 conflict → any snackbar should appear (server-side error)
            const snackbar = authenticatedPage.locator('.v-snackbar')
            await expect(snackbar).toBeVisible({ timeout: 10_000 })
        })
    })
})

test.describe('Validation errors — API key creation', () => {
    test.beforeEach(async ({ authenticatedPage }) => {
        const nav = new NavigationComponent(authenticatedPage)
        await nav.navigateTo('/api-keys')
    })

    test('empty name prevents submission or shows validation error', async ({
        authenticatedPage,
    }) => {
        const apiKeys = new ApiKeysPage(authenticatedPage)
        await apiKeys.clickCreate()

        const dialog = authenticatedPage.locator('.v-dialog--active, .v-overlay--active .v-card')
        await expect(dialog).toBeVisible({ timeout: 10_000 })

        // Clear the name field to make it empty
        await dialog.getByLabel(/name/i).fill('')
        // Click somewhere else to trigger blur/validation
        await dialog.getByLabel(/name/i).blur()

        // Create button should be disabled when name is empty
        const createBtn = dialog.getByRole('button', { name: /create|save|submit/i })
        const btnDisabled = await createBtn.isDisabled().catch(() => true)

        if (btnDisabled) {
            // Button is disabled — validation is working, no need to click
            await expect(createBtn).toBeDisabled({ timeout: 5_000 })
        } else {
            // Button is enabled — click and expect an error response
            await createBtn.click()
            // Either a field error via [role="alert"] OR dialog stays open
            const alertMsg = dialog.locator('[role="alert"]')
            const snackbar = authenticatedPage.locator('.v-snackbar')
            await expect(alertMsg.first().or(snackbar)).toBeVisible({ timeout: 10_000 })
        }

        await expect(dialog).toBeVisible()
    })
})

test.describe('Validation errors — role creation', () => {
    test.beforeEach(async ({ authenticatedPage }) => {
        const nav = new NavigationComponent(authenticatedPage)
        await nav.navigateTo('/permissions')
        await authenticatedPage.getByTestId('permissions-roles-tab').click()
    })

    test.describe.serial('duplicate role name shows conflict error', () => {
        test('create first role, then second with same name shows error', async ({
            authenticatedPage,
        }) => {
            const permissions = new PermissionsPage(authenticatedPage)
            const suffix = Date.now()
            const roleName = `e2e_duprole_${suffix}`

            // --- Create the first role ---
            await permissions.createRole(roleName, 'First role in duplicate test')
            const firstDialog = authenticatedPage.locator(
                '.v-dialog--active, .v-overlay--active .v-card'
            )
            await expect(firstDialog).not.toBeVisible({ timeout: 10_000 })

            await permissions.expectRoleInTable(roleName)

            // --- Attempt to create second role with same name ---
            await authenticatedPage.getByTestId('roles-create-btn').click()
            const dialog = authenticatedPage.locator(
                '.v-dialog--active, .v-overlay--active .v-card'
            )
            await expect(dialog).toBeVisible({ timeout: 10_000 })

            await dialog.getByLabel(/name/i).fill(roleName) // duplicate
            await dialog.getByLabel(/description/i).fill('Duplicate role attempt')
            await dialog.getByRole('button', { name: /create|save|submit/i }).click()

            // Expect a 409 conflict → any snackbar should appear (server-side error)
            const snackbar = authenticatedPage.locator('.v-snackbar')
            await expect(snackbar).toBeVisible({ timeout: 10_000 })
        })
    })
})
