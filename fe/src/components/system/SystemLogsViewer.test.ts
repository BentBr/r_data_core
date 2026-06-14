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
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import SystemLogsViewer from './SystemLogsViewer.vue'
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

vi.mock('@/composables/useErrorHandler', () => ({
    useErrorHandler: () => ({ handleError: vi.fn() }),
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
