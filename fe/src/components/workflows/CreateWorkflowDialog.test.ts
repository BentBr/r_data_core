import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import CreateWorkflowDialog from './CreateWorkflowDialog.vue'

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        previewCron: vi.fn().mockResolvedValue(['2025-01-01T00:00:00Z']),
        validateDsl: vi.fn().mockResolvedValue({ ok: true }),
        createWorkflow: vi.fn().mockResolvedValue({ uuid: '019a46aa-582d-7f51-8782-641a00ec534c' }),
        // Needed by nested DslConfigurator
        getDslFromOptions: vi.fn().mockResolvedValue({}),
        getDslToOptions: vi.fn().mockResolvedValue({}),
        getDslTransformOptions: vi.fn().mockResolvedValue({}),
        getEntityDefinitions: vi
            .fn()
            .mockResolvedValue({ data: [], meta: { pagination: { total: 0 } } }),
        getEntityFields: vi.fn().mockResolvedValue([]),
    },
    ValidationError: class ValidationError extends Error {
        violations: Array<{ field: string; message: string }>
        constructor(violations: Array<{ field: string; message: string }>) {
            super('validation')
            this.violations = violations
        }
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k.split('.').pop() }),
}))

describe('CreateWorkflowDialog', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    it('submits with DSL steps and emits created', async () => {
        const wrapper = mount(CreateWorkflowDialog, {
            props: { modelValue: true },
        })

        // Inject DSL via child configurator emit to avoid relying on expansion/textarea rendering
        const dsl = [
            {
                from: {
                    type: 'csv',
                    uri: 'http://example.com/data.csv',
                    options: { header: true },
                    mapping: {},
                },
                transform: { type: 'none' },
                to: { type: 'json', output: 'api', mapping: {} },
            },
        ]
        // Set steps directly via exposed API
        ;(wrapper.vm as any).steps = dsl
        await nextTick()

        await (wrapper.vm as any).submit()

        const emitted = wrapper.emitted('created') as any[]
        expect(emitted?.length).toBe(1)
        expect(emitted[0][0]).toBeTypeOf('string')
    })

    it('shows validation error when DSL missing', async () => {
        const wrapper = mount(CreateWorkflowDialog, {
            props: { modelValue: true },
        })
        // Trigger submit without providing steps/DSL
        await (wrapper.vm as any).submit()

        // Expect validation state set
        expect((wrapper.vm as any).configError?.toLowerCase()?.includes('dsl_required')).toBe(true)
    })

    it('disables cron field when from.api source is used', async () => {
        const wrapper = mount(CreateWorkflowDialog, {
            props: { modelValue: true },
        })

        // Set steps with from.api source (accepts POST)
        const dsl = [
            {
                from: {
                    type: 'format',
                    source: {
                        source_type: 'api',
                        config: {}, // No endpoint field = accepts POST
                        auth: null,
                    },
                    format: {
                        format_type: 'csv',
                        options: { has_header: true },
                    },
                    mapping: {},
                },
                transform: { type: 'none' },
                to: {
                    type: 'format',
                    output: { mode: 'api' },
                    format: {
                        format_type: 'json',
                        options: {},
                    },
                    mapping: {},
                },
            },
        ]
        ;(wrapper.vm as any).steps = dsl
        await nextTick()

        // Find the cron field by label
        const cronField = wrapper.find('input[type="text"]')
        if (cronField.exists()) {
            // Check if cron field is disabled
            expect(cronField.attributes('disabled')).toBeDefined()

            // Check if hint is shown
            const hint = wrapper.text()
            expect(hint).toContain('cron_disabled_for_api_source')
        } else {
            // If field not found, check that hasApiSource computed is true
            expect((wrapper.vm as any).hasApiSource).toBe(true)
        }
    })

    it('enables cron field when from.api source is not used', async () => {
        const wrapper = mount(CreateWorkflowDialog, {
            props: { modelValue: true },
        })

        // Set steps with from.uri source (not from.api)
        const dsl = [
            {
                from: {
                    type: 'format',
                    source: {
                        source_type: 'uri',
                        config: {
                            uri: 'http://example.com/data.csv',
                        },
                        auth: null,
                    },
                    format: {
                        format_type: 'csv',
                        options: { has_header: true },
                    },
                    mapping: {},
                },
                transform: { type: 'none' },
                to: {
                    type: 'format',
                    output: { mode: 'api' },
                    format: {
                        format_type: 'json',
                        options: {},
                    },
                    mapping: {},
                },
            },
        ]
        ;(wrapper.vm as any).steps = dsl
        await nextTick()

        // Check that hasApiSource is false
        expect((wrapper.vm as any).hasApiSource).toBe(false)

        // Check if cron field is NOT disabled (if found)
        const cronInput = wrapper.find('input[type="text"]')
        if (cronInput.exists()) {
            expect(cronInput.attributes('disabled')).toBeUndefined()
        }
    })
})
