import { expect, request as pwRequest, type Page } from '@playwright/test'
import { test } from '../fixtures/auth.fixture'
import { login } from '../helpers/api-client'
import { NavigationComponent } from '../page-objects/components/navigation.component'
import { WorkflowsPage } from '../page-objects/workflows.page'

// The seam neither unit nor integration tests cover: an admin ticks the box in
// the admin UI and the workflow's public endpoint starts refusing traffic.
//
// Unlike zz-rate-limit.test.ts, which trips the shared admin-login limiter for
// the whole runner IP, this counter is scoped to one workflow UUID and vanishes
// with it - so this spec does not need to run last.

const API_BASE_URL = process.env.E2E_API_BASE_URL ?? 'http://rdatacore.docker'
const WORKFLOW_NAME = 'e2e_rate_limited_workflow'
const MAX_REQUESTS = 2

/** Configuration for a provider workflow that answers on the public endpoint. */
const providerConfig = {
    steps: [
        {
            from: {
                type: 'format',
                source: { source_type: 'api', config: {} },
                format: { format_type: 'json', options: {} },
                mapping: { title: 'title' },
            },
            transform: { type: 'none' },
            to: {
                type: 'format',
                output: { mode: 'api' },
                format: { format_type: 'json', options: {} },
                mapping: { title: 'title' },
            },
        },
    ],
}

let token = ''
let workflowUuid = ''

async function adminRequest(
    method: string,
    path: string,
    body?: Record<string, unknown>
): Promise<Response> {
    return fetch(`${API_BASE_URL}/admin/api/v1${path}`, {
        method,
        headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
        body: body ? JSON.stringify(body) : undefined,
    })
}

/**
 * Call the workflow's public endpoint and report the status.
 *
 * The call is deliberately unauthenticated: the limiter runs ahead of
 * authentication, so a refused caller is turned away before any key is checked.
 * That makes the meaningful assertion 429 vs. anything-else, not 200.
 */
async function callPublicEndpoint(): Promise<number> {
    const ctx = await pwRequest.newContext({ baseURL: API_BASE_URL })
    const res = await ctx.get(`/api/v1/workflows/${workflowUuid}`)
    const status = res.status()
    await ctx.dispose()
    return status
}

/** Open the workflow's edit dialog with the config panel expanded. */
async function openConfigPanel(page: Page) {
    const nav = new NavigationComponent(page)
    await nav.navigateTo('/workflows')

    const workflows = new WorkflowsPage(page)
    await workflows.expectTableVisible()
    await workflows.editWorkflow(WORKFLOW_NAME)

    const dialog = page.locator('.v-overlay--active .v-card').first()
    await expect(dialog).toBeVisible({ timeout: 10_000 })

    // The rate-limit controls live inside the collapsed config panel.
    await dialog.locator('.v-expansion-panel-title').first().click()
    await expect(dialog.getByTestId('rate-limit-enabled')).toBeVisible({ timeout: 10_000 })

    return dialog
}

async function saveDialog(page: Page) {
    const dialog = page.locator('.v-overlay--active .v-card').first()
    await dialog.getByRole('button', { name: 'Save' }).click()
    await expect(dialog).not.toBeVisible({ timeout: 15_000 })
}

test.describe.serial('Per-workflow rate limiting', () => {
    test.beforeAll(async () => {
        token = await login()
        const res = await adminRequest('POST', '/workflows', {
            name: WORKFLOW_NAME,
            description: 'E2E per-workflow rate limit',
            kind: 'provider',
            enabled: true,
            config: providerConfig,
            versioning_disabled: false,
        })
        if (!res.ok) {
            throw new Error(`Create workflow failed (${res.status}): ${await res.text()}`)
        }
        const body = (await res.json()) as { data: { uuid: string } }
        workflowUuid = body.data.uuid
    })

    test.afterAll(async () => {
        if (workflowUuid) {
            await adminRequest('DELETE', `/workflows/${workflowUuid}`)
        }
    })

    test('a workflow with no limit is not throttled', async () => {
        for (let attempt = 1; attempt <= MAX_REQUESTS + 3; attempt++) {
            expect(
                await callPublicEndpoint(),
                `attempt ${attempt} on an unlimited workflow`
            ).not.toBe(429)
        }
    })

    test('ticking the box in the UI makes the endpoint return 429', async ({
        authenticatedPage,
    }) => {
        const dialog = await openConfigPanel(authenticatedPage)

        await dialog.getByTestId('rate-limit-enabled').locator('input').check()
        await dialog.getByTestId('rate-limit-max').locator('input').fill(String(MAX_REQUESTS))
        await dialog.getByTestId('rate-limit-window').locator('input').fill('60')
        await saveDialog(authenticatedPage)

        for (let attempt = 1; attempt <= MAX_REQUESTS; attempt++) {
            expect(await callPublicEndpoint(), `attempt ${attempt} is inside the budget`).not.toBe(
                429
            )
        }

        expect(
            await callPublicEndpoint(),
            `request ${MAX_REQUESTS + 1} exceeds a budget of ${MAX_REQUESTS}`
        ).toBe(429)
    })

    // The edit path, and the end-to-end proof that cache invalidation works:
    // a stale cached config would keep throttling after the switch is off.
    test('unticking it restores unthrottled access', async ({ authenticatedPage }) => {
        const dialog = await openConfigPanel(authenticatedPage)

        await dialog.getByTestId('rate-limit-enabled').locator('input').uncheck()
        await saveDialog(authenticatedPage)

        for (let attempt = 1; attempt <= 10; attempt++) {
            expect(
                await callPublicEndpoint(),
                `attempt ${attempt} after the limit was removed`
            ).not.toBe(429)
        }
    })
})
