import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import CreateWorkflowDialog from './CreateWorkflowDialog.vue'

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        previewCron: vi.fn().mockResolvedValue(['2025-01-01T00:00:00Z']),
        validateDsl: vi.fn().mockResolvedValue({ ok: true }),
        createWorkflow: vi.fn().mockResolvedValue({ uuid: '019a46aa-582d-7f51-8782-641a00ec534c' }),
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

    it.skip('submits with DSL steps and emits created', async () => {
        const wrapper = mount(CreateWorkflowDialog, {
            props: { modelValue: true },
        })

        // Open the config expansion (button contains 'config_label')
        const expander = wrapper.findAll('button').find(b => /config_label/i.test(b.text()))
        if (expander) {
            await expander.trigger('click')
        }
        // Fill config JSON textarea with valid steps
        const textarea = wrapper.findAll('textarea').at(-1)
        expect(textarea).toBeTruthy()
        await textarea!.setValue(
            JSON.stringify(
                {
                    steps: [
                        {
                            from: { type: 'csv', uri: 'http://example.com/data.csv', mapping: {} },
                            transform: { type: 'none' },
                            to: { type: 'json', output: 'api', mapping: {} },
                        },
                    ],
                },
                null,
                2
            )
        )

        // Click create button via VBtn component
        const createBtn = wrapper.findAll('button').find(b => /create_button/i.test(b.text()))
        expect(createBtn).toBeTruthy()
        await createBtn!.trigger('click')

        const emitted = wrapper.emitted('created') as any[]
        expect(emitted?.length).toBe(1)
        expect(emitted[0][0]).toBeTypeOf('string')
    })

    it.skip('shows validation error when DSL missing', async () => {
        const wrapper = mount(CreateWorkflowDialog, {
            props: { modelValue: true },
        })
        // Trigger submit without providing steps/DSL
        const createBtn = wrapper.findAll('button').find(b => /create_button/i.test(b.text()))
        expect(createBtn).toBeTruthy()
        await createBtn!.trigger('click')

        const text = wrapper.text().toLowerCase()
        // Expect an error string from translations: 'dsl_required'
        expect(text.includes('dsl_required')).toBe(true)
    })
})


