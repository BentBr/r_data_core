/**
 * Security regression test: email preview must render in a sandboxed iframe,
 * never via v-html, to prevent stored-XSS execution.
 *
 * Mount strategy: Vuetify VDialog uses a teleport to document.body.
 * The global test-setup stubs <teleport> which hides dialog content.
 * We override that stub per-test (stubs: { teleport: false }) and attach
 * the wrapper to a real DOM node so the teleported content is queryable
 * via document.body.
 */
import { describe, it, expect, vi, beforeEach, afterEach, type Mock } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import SystemLogsViewer from './SystemLogsViewer.vue'
import { typedHttpClient } from '@/api/typed-client'
import type { SystemLog } from '@/api/clients/system-logs'

// ---------------------------------------------------------------------------
// Network stubs — no real HTTP
// ---------------------------------------------------------------------------
vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        listSystemLogs: vi.fn().mockResolvedValue({
            data: [],
            meta: { pagination: { total: 0 } },
        }),
    },
    ValidationError: class ValidationError extends Error {
        violations: Array<{ field: string; message: string }>
        constructor(violations: Array<{ field: string; message: string }>) {
            super('validation')
            this.violations = violations
        }
    },
}))

const mockHandleError: Mock = vi.fn()
vi.mock('@/composables/useErrorHandler', () => ({
    useErrorHandler: () => ({ handleError: mockHandleError }),
}))

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Build a minimal valid email_sent log. */
const makeEmailLog = (details: Record<string, unknown>): SystemLog => ({
    uuid: 'aaaaaaaa-0000-0000-0000-000000000001',
    created_at: '2026-01-01T00:00:00Z',
    created_by: null,
    status: 'success',
    log_type: 'email_sent',
    resource_type: 'email',
    resource_uuid: null,
    summary: 'Test email sent',
    details,
})

/**
 * Mount the component attached to a fresh div in document.body, overriding
 * the global teleport stub so Vuetify's dialog teleport actually renders.
 * Returns the wrapper and a cleanup callback.
 */
const mountAttached = () => {
    const container = document.createElement('div')
    document.body.appendChild(container)

    const wrapper = mount(SystemLogsViewer, {
        attachTo: container,
        global: {
            // Override the global teleport: true stub from test-setup.ts.
            // This lets Vuetify teleport dialog content into document.body.
            stubs: { teleport: false },
        },
    })

    return { wrapper, container }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('SystemLogsViewer — email preview XSS hardening', () => {
    let cleanup: (() => void) | undefined

    beforeEach(() => {
        vi.clearAllMocks()
        cleanup = undefined
    })

    afterEach(() => {
        cleanup?.()
    })

    /**
     * Open the detail dialog for a given log by calling the component's
     * internal openDetailView method, then wait for the next tick so Vue
     * flushes the reactive updates and Vuetify renders the dialog.
     */
    const openDetail = async (log: SystemLog) => {
        const { wrapper, container: c } = mountAttached()

        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        const vm = wrapper.vm as unknown as {
            openDetailView: (log: SystemLog) => void
        }
        vm.openDetailView(log)

        // Give Vuetify time to render the overlay / dialog into body
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        cleanup = () => {
            wrapper.unmount()
            if (document.body.contains(c)) document.body.removeChild(c)
        }

        return wrapper
    }

    it('renders a sandboxed iframe (not v-html) for an email_sent log with body_html', async () => {
        const xssPayload = '<b>hello</b><script>alert(1)<\/script>'
        await openDetail(makeEmailLog({ body_html: xssPayload }))

        const iframe = document.body.querySelector<HTMLIFrameElement>('iframe.email-preview-frame')
        expect(iframe).not.toBeNull()

        // sandbox="" = all sandbox restrictions active (no scripts, no navigation)
        expect(iframe!.getAttribute('sandbox')).toBe('')

        // The XSS payload lives only in the srcdoc attribute string, never parsed into live DOM
        expect(iframe!.getAttribute('srcdoc')).toContain(xssPayload)
    })

    it('sandbox attribute is exactly "" — no permission tokens granted', async () => {
        await openDetail(makeEmailLog({ body_html: '<p>safe content</p>' }))

        const iframe = document.body.querySelector<HTMLIFrameElement>('iframe.email-preview-frame')
        expect(iframe).not.toBeNull()

        const sandboxValue = iframe!.getAttribute('sandbox') ?? 'MISSING'
        expect(sandboxValue).toBe('')
        expect(sandboxValue).not.toContain('allow-scripts')
        expect(sandboxValue).not.toContain('allow-same-origin')
        expect(sandboxValue).not.toContain('allow-forms')
        expect(sandboxValue).not.toContain('allow-top-navigation')
    })

    it('does NOT render the script payload as live DOM — no <script> tag in component tree', async () => {
        const scriptContent = 'alert(1)'
        const wrapper = await openDetail(
            makeEmailLog({ body_html: `<b>ok</b><script>${scriptContent}<\/script>` })
        )

        // No <script> element should exist anywhere in the rendered component DOM
        const scriptTags = wrapper.findAll('script')
        expect(scriptTags).toHaveLength(0)

        // Also verify document.body has no live script tag (iframe srcdoc is not parsed into DOM)
        const bodyScripts = document.body.querySelectorAll('script')
        expect(bodyScripts).toHaveLength(0)
    })

    it('falls back to details.html when body_html is absent', async () => {
        const htmlContent = '<p>from html field</p>'
        await openDetail(makeEmailLog({ html: htmlContent }))

        const iframe = document.body.querySelector<HTMLIFrameElement>('iframe.email-preview-frame')
        expect(iframe).not.toBeNull()
        expect(iframe!.getAttribute('srcdoc')).toContain(htmlContent)
    })

    it('does NOT render the iframe for a non-email log type', async () => {
        const nonEmailLog: SystemLog = {
            uuid: 'cccccccc-0000-0000-0000-000000000003',
            created_at: '2026-01-01T00:00:00Z',
            created_by: null,
            status: 'success',
            log_type: 'entity_created',
            resource_type: 'admin_user',
            resource_uuid: null,
            summary: 'entity created',
            details: { body_html: '<b>not an email</b>' },
        }
        await openDetail(nonEmailLog)

        const iframe = document.body.querySelector('iframe.email-preview-frame')
        expect(iframe).toBeNull()
    })

    // -----------------------------------------------------------------------
    // Unit tests for getHtmlPreview — no DOM mount needed
    // -----------------------------------------------------------------------
    describe('getHtmlPreview helper', () => {
        it('returns body_html when both body_html and html are present', async () => {
            const { wrapper } = mountAttached()
            cleanup = () => wrapper.unmount()

            const vm = wrapper.vm as unknown as {
                getHtmlPreview: (details: unknown) => string
            }
            expect(vm.getHtmlPreview({ body_html: '<b>body</b>', html: '<i>html</i>' })).toBe(
                '<b>body</b>'
            )
        })

        it('returns html when body_html is absent', async () => {
            const { wrapper } = mountAttached()
            cleanup = () => wrapper.unmount()

            const vm = wrapper.vm as unknown as {
                getHtmlPreview: (details: unknown) => string
            }
            expect(vm.getHtmlPreview({ html: '<i>only html</i>' })).toBe('<i>only html</i>')
        })

        it('returns empty string for null details', async () => {
            const { wrapper } = mountAttached()
            cleanup = () => wrapper.unmount()

            const vm = wrapper.vm as unknown as {
                getHtmlPreview: (details: unknown) => string
            }
            expect(vm.getHtmlPreview(null)).toBe('')
        })
    })
})

// ---------------------------------------------------------------------------
// Filtering, pagination, table rendering, and pure-function branches
// ---------------------------------------------------------------------------

type VmInternals = {
    getLogTypeColor: (logType: string) => string
    getStatusColor: (status: string) => string
    toIso8601: (dateStr: string | null) => string | undefined
    formatDate: (dateString: string) => string
    onRowClick: (event: unknown, row: { item: SystemLog }) => void
    detailLog: SystemLog | null
    showDetail: boolean
    logs: SystemLog[]
    totalItems: number
    loading: boolean
    page: number
    itemsPerPage: number
    filterResourceUuid: string | null
    filterDateFrom: string | null
    handleFilterChange: () => void
    handleResourceUuidSearch: () => void
    handlePageChange: (page: number) => void
    handleItemsPerPageChange: (size: number) => void
}

const listSystemLogsMock = typedHttpClient.listSystemLogs as Mock

// Stub the interactive Vuetify filter/table controls: assigning to the
// underlying refs directly (e.g. `vm.page = 3`) makes these controlled
// components re-emit their own update events to resync, which double-fires
// the very handlers under test. These tests only care about the exposed
// script-setup state, not the rendered table.
const mountPlain = () =>
    mount(SystemLogsViewer, {
        global: {
            stubs: {
                VDataTableServer: true,
                VSelect: true,
                VTextField: true,
            },
        },
    })

describe('SystemLogsViewer — pure helper functions', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        listSystemLogsMock.mockResolvedValue({ data: [], meta: { pagination: { total: 0 } } })
    })

    it('maps every known log type to its colour, with a default for the rest', async () => {
        const wrapper = mountPlain()
        const vm = wrapper.vm as unknown as VmInternals

        expect(vm.getLogTypeColor('email_sent')).toBe('blue')
        expect(vm.getLogTypeColor('entity_created')).toBe('success')
        expect(vm.getLogTypeColor('entity_updated')).toBe('warning')
        expect(vm.getLogTypeColor('entity_deleted')).toBe('error')
        expect(vm.getLogTypeColor('auth_event')).toBe('purple')
        expect(vm.getLogTypeColor('something_unexpected')).toBe('grey')
    })

    it('maps every known status to its colour, with a default for the rest', async () => {
        const wrapper = mountPlain()
        const vm = wrapper.vm as unknown as VmInternals

        expect(vm.getStatusColor('success')).toBe('success')
        expect(vm.getStatusColor('failed')).toBe('error')
        expect(vm.getStatusColor('pending')).toBe('warning')
        expect(vm.getStatusColor('something_unexpected')).toBe('grey')
    })

    it('converts a datetime-local value to UTC ISO 8601', async () => {
        const wrapper = mountPlain()
        const vm = wrapper.vm as unknown as VmInternals

        expect(vm.toIso8601(null)).toBeUndefined()
        expect(vm.toIso8601('not-a-date')).toBeUndefined()

        const converted = vm.toIso8601('2026-04-19T13:04')
        expect(converted).toBe(new Date('2026-04-19T13:04').toISOString())
    })

    it('formats a valid date using the locale string', async () => {
        const wrapper = mountPlain()
        const vm = wrapper.vm as unknown as VmInternals

        expect(vm.formatDate('2026-01-01T00:00:00Z')).toBe(
            new Date('2026-01-01T00:00:00Z').toLocaleString()
        )
    })

    it('falls back to the raw string when formatting throws', async () => {
        const wrapper = mountPlain()
        const vm = wrapper.vm as unknown as VmInternals
        const spy = vi.spyOn(Date.prototype, 'toLocaleString').mockImplementation(() => {
            throw new Error('boom')
        })

        expect(vm.formatDate('2026-01-01T00:00:00Z')).toBe('2026-01-01T00:00:00Z')

        spy.mockRestore()
    })

    it('opens the detail view for the row that was clicked', async () => {
        const wrapper = mountPlain()
        const vm = wrapper.vm as unknown as VmInternals
        const row: SystemLog = makeEmailLog({})

        vm.onRowClick(null, { item: row })
        await nextTick()

        expect(vm.detailLog).toEqual(row)
        expect(vm.showDetail).toBe(true)
    })

    it('resets the page and reloads when a dropdown filter changes', async () => {
        const wrapper = mountPlain()
        const vm = wrapper.vm as unknown as VmInternals
        await nextTick()
        listSystemLogsMock.mockClear()
        vm.page = 5

        vm.handleFilterChange()
        await nextTick()

        expect(vm.page).toBe(1)
        expect(listSystemLogsMock).toHaveBeenCalled()
    })

    it('debounces the resource UUID search before reloading', async () => {
        const wrapper = mountPlain()
        const vm = wrapper.vm as unknown as VmInternals
        await nextTick()
        listSystemLogsMock.mockClear()
        vm.page = 3
        vm.filterResourceUuid = 'abc-123'

        vm.handleResourceUuidSearch()
        // A second call within the debounce window cancels the first timer.
        vm.handleResourceUuidSearch()
        expect(listSystemLogsMock).not.toHaveBeenCalled()

        await new Promise(resolve => setTimeout(resolve, 450))

        expect(vm.page).toBe(1)
        expect(listSystemLogsMock).toHaveBeenCalledTimes(1)
        expect(listSystemLogsMock).toHaveBeenCalledWith(
            expect.objectContaining({ resource_uuid: 'abc-123' })
        )
    })

    it('reloads with the requested page on page change', async () => {
        const wrapper = mountPlain()
        const vm = wrapper.vm as unknown as VmInternals
        await nextTick()
        listSystemLogsMock.mockClear()

        vm.handlePageChange(4)
        await nextTick()

        expect(vm.page).toBe(4)
        expect(listSystemLogsMock).toHaveBeenCalledWith(expect.objectContaining({ page: 4 }))
    })

    it('resets to page 1 and reloads when items-per-page changes', async () => {
        const wrapper = mountPlain()
        const vm = wrapper.vm as unknown as VmInternals
        await nextTick()
        vm.page = 2
        listSystemLogsMock.mockClear()

        vm.handleItemsPerPageChange(50)
        await nextTick()

        expect(vm.page).toBe(1)
        expect(vm.itemsPerPage).toBe(50)
        expect(listSystemLogsMock).toHaveBeenCalledWith(
            expect.objectContaining({ page_size: 50, page: 1 })
        )
    })

    it('converts filterDateFrom/filterDateTo into ISO params sent to the API', async () => {
        const wrapper = mountPlain()
        const vm = wrapper.vm as unknown as VmInternals
        await nextTick()
        listSystemLogsMock.mockClear()
        vm.filterDateFrom = '2026-04-19T13:04'

        vm.handleFilterChange()
        await nextTick()

        expect(listSystemLogsMock).toHaveBeenCalledWith(
            expect.objectContaining({ date_from: new Date('2026-04-19T13:04').toISOString() })
        )
    })

    it('clears the list and surfaces the error when loading fails', async () => {
        listSystemLogsMock.mockRejectedValueOnce(new Error('network down'))
        const wrapper = mountPlain()
        await new Promise(resolve => setTimeout(resolve, 10))
        const vm = wrapper.vm as unknown as VmInternals

        expect(mockHandleError).toHaveBeenCalled()
        expect(vm.logs).toEqual([])
        expect(vm.totalItems).toBe(0)
        expect(vm.loading).toBe(false)
    })
})

describe('SystemLogsViewer — table row rendering', () => {
    const sampleLogs: SystemLog[] = [
        {
            uuid: 'uuid-1',
            created_at: '2026-01-01T10:00:00Z',
            created_by: 'admin-1',
            status: 'pending',
            log_type: 'entity_updated',
            resource_type: 'admin_user',
            resource_uuid: '11111111-aaaa-bbbb-cccc-111111111111',
            summary: 'Updated a user',
            details: null,
        },
        {
            uuid: 'uuid-2',
            created_at: '2026-01-02T10:00:00Z',
            created_by: null,
            status: 'failed',
            // Cast to simulate a resource type the route map does not know
            // about yet — defensive branch for forward compatibility.
            log_type: 'auth_event',
            resource_type: 'unmapped_future_type' as unknown as SystemLog['resource_type'],
            resource_uuid: '22222222-aaaa-bbbb-cccc-222222222222',
            summary: 'Login failed',
            details: null,
        },
        {
            uuid: 'uuid-3',
            created_at: '2026-01-03T10:00:00Z',
            created_by: null,
            status: 'success',
            log_type: 'entity_deleted',
            resource_type: 'workflow',
            resource_uuid: null,
            summary: 'Deleted a workflow',
            details: null,
        },
    ]

    // Unlike mountPlain(), these need the real table rendered.
    const mountWithRealTable = () => mount(SystemLogsViewer)

    beforeEach(() => {
        vi.clearAllMocks()
        listSystemLogsMock.mockResolvedValue({
            data: sampleLogs,
            meta: { pagination: { total: 3 } },
        })
    })

    it('renders a row per log with its type, resource, summary, and status', async () => {
        const wrapper = mountWithRealTable()
        await new Promise(resolve => setTimeout(resolve, 100))
        await nextTick()

        const table = wrapper.find('[data-testid="system-logs-table"]')
        expect(table.text()).toContain('entity_updated')
        expect(table.text()).toContain('admin_user')
        expect(table.text()).toContain('Updated a user')
        expect(table.text()).toContain('pending')
        expect(table.text()).toContain('Login failed')
        expect(table.text()).toContain('Deleted a workflow')
    })

    it('links a known resource type to its route', async () => {
        const wrapper = mountWithRealTable()
        await new Promise(resolve => setTimeout(resolve, 100))
        await nextTick()

        const link = wrapper.find('router-link')
        expect(link.exists()).toBe(true)
        expect(link.text()).toContain('11111111')
    })

    it('shows a plain code element for a resource type without a route mapping', async () => {
        const wrapper = mountWithRealTable()
        await new Promise(resolve => setTimeout(resolve, 100))
        await nextTick()

        const table = wrapper.find('[data-testid="system-logs-table"]')
        expect(table.text()).toContain('22222222')
    })

    it('shows a dash when the log has no resource UUID', async () => {
        const wrapper = mountWithRealTable()
        await new Promise(resolve => setTimeout(resolve, 100))
        await nextTick()

        const table = wrapper.find('[data-testid="system-logs-table"]')
        expect(table.text()).toContain('—')
    })

    describe('detail dialog resource/actor branches (real teleport)', () => {
        let cleanup: (() => void) | undefined

        afterEach(() => {
            cleanup?.()
            cleanup = undefined
        })

        it('links a known resource type to its route inside the detail dialog', async () => {
            const { wrapper, container } = mountAttached()
            await new Promise(resolve => setTimeout(resolve, 50))
            const vm = wrapper.vm as unknown as VmInternals
            vm.onRowClick(null, { item: sampleLogs[0] })
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))
            cleanup = () => {
                wrapper.unmount()
                if (document.body.contains(container)) document.body.removeChild(container)
            }

            expect(document.body.textContent).toContain('11111111-aaaa-bbbb-cccc-111111111111')
            expect(document.body.textContent).toContain('admin-1')
        })

        it('shows a plain code element (no link) for an unmapped resource type', async () => {
            const { wrapper, container } = mountAttached()
            await new Promise(resolve => setTimeout(resolve, 50))
            const vm = wrapper.vm as unknown as VmInternals
            vm.onRowClick(null, { item: sampleLogs[1] })
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))
            cleanup = () => {
                wrapper.unmount()
                if (document.body.contains(container)) document.body.removeChild(container)
            }

            expect(document.body.textContent).toContain('22222222-aaaa-bbbb-cccc-222222222222')
        })

        it('shows a dash for a log with neither resource UUID nor actor', async () => {
            const { wrapper, container } = mountAttached()
            await new Promise(resolve => setTimeout(resolve, 50))
            const vm = wrapper.vm as unknown as VmInternals
            vm.onRowClick(null, { item: sampleLogs[2] })
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))
            cleanup = () => {
                wrapper.unmount()
                if (document.body.contains(container)) document.body.removeChild(container)
            }

            const dashCount = (document.body.textContent?.match(/—/g) ?? []).length
            expect(dashCount).toBeGreaterThanOrEqual(2)
        })
    })
})
