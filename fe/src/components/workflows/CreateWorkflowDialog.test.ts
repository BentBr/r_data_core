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
        getEntityDefinitions: vi.fn().mockResolvedValue({ data: [], meta: { pagination: { total: 0 } } }),
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
                from: { type: 'csv', uri: 'http://example.com/data.csv', options: { header: true }, mapping: {} },
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
})


