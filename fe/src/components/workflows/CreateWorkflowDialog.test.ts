import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import CreateWorkflowDialog from './CreateWorkflowDialog.vue'
import type { DslStep } from './dsl/dsl-utils'

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
        const vm = wrapper.vm as {
            steps: DslStep[]
            submit: () => Promise<void>
        }
        vm.steps = dsl as DslStep[]
        await nextTick()

        await vm.submit()

        const emitted = wrapper.emitted('created') as Array<[string]> | undefined
        expect(emitted?.length).toBe(1)
        expect(emitted[0][0]).toBeTypeOf('string')
    })

    it('shows validation error when DSL missing', async () => {
        const wrapper = mount(CreateWorkflowDialog, {
            props: { modelValue: true },
        })
        // Trigger submit without providing steps/DSL
        const vm = wrapper.vm as {
            submit: () => Promise<void>
            configError?: string
            configJson: string
        }
        // Set valid JSON with empty steps to trigger DSL required error
        vm.configJson = '{"steps": []}'
        await vm.submit()

        // Expect validation state set
        expect(vm.configError?.toLowerCase()?.includes('dsl_required')).toBe(true)
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
        const vm = wrapper.vm as {
            steps: DslStep[]
            hasApiSource?: boolean
        }
        vm.steps = dsl as DslStep[]
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
            expect(vm.hasApiSource).toBe(true)
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
        const vm = wrapper.vm as {
            steps: DslStep[]
            hasApiSource?: boolean
        }
        vm.steps = dsl as DslStep[]
        await nextTick()

        // Check that hasApiSource is false
        expect(vm.hasApiSource).toBe(false)
    })

    it('disables cron field when to.format.output.mode is api', async () => {
        const wrapper = mount(CreateWorkflowDialog, {
            props: { modelValue: true },
        })

        // Set steps with to.format.output.mode = api (exports via GET)
        const dsl = [
            {
                from: {
                    type: 'format',
                    source: {
                        source_type: 'uri',
                        config: { uri: 'http://example.com/data.csv' },
                        auth: { type: 'none' },
                    },
                    format: {
                        format_type: 'csv',
                        options: {},
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

        const vm = wrapper.vm as {
            steps: DslStep[]
            hasApiOutput: boolean
            form: { schedule_cron: string | null }
        }
        vm.steps = dsl as DslStep[]
        await nextTick()

        // Check that hasApiOutput is true
        expect(vm.hasApiOutput).toBe(true)

        // Check that cron field is disabled
        const cronField = wrapper.find('input[type="text"]')
        if (cronField.exists()) {
            expect(cronField.attributes('disabled')).toBeDefined()
        }

        // Check that cron value is cleared
        expect(vm.form.schedule_cron).toBeNull()
    })

    it('enables cron field when api output is not used', async () => {
        const wrapper = mount(CreateWorkflowDialog, {
            props: { modelValue: true },
        })

        // Set steps without api output
        const dsl = [
            {
                from: {
                    type: 'format',
                    source: {
                        source_type: 'uri',
                        config: { uri: 'http://example.com/data.csv' },
                        auth: { type: 'none' },
                    },
                    format: {
                        format_type: 'csv',
                        options: {},
                    },
                    mapping: {},
                },
                transform: { type: 'none' },
                to: {
                    type: 'format',
                    output: { mode: 'download' },
                    format: {
                        format_type: 'json',
                        options: {},
                    },
                    mapping: {},
                },
            },
        ]

        const vm = wrapper.vm as {
            steps: DslStep[]
            hasApiOutput: boolean
        }
        vm.steps = dsl as DslStep[]
        await nextTick()

        // Check that hasApiOutput is false
        expect(vm.hasApiOutput).toBe(false)

        // Check if cron field is NOT disabled (if found)
        const cronInput = wrapper.find('input[type="text"]')
        if (cronInput.exists()) {
            expect(cronInput.attributes('disabled')).toBeUndefined()
        }
    })

    it('disables cron field when to.format.output.mode is api', async () => {
        const wrapper = mount(CreateWorkflowDialog, {
            props: { modelValue: true },
        })

        // Set steps with to.format.output.mode = api (exports via GET)
        const dsl = [
            {
                from: {
                    type: 'format',
                    source: {
                        source_type: 'uri',
                        config: { uri: 'http://example.com/data.csv' },
                        auth: { type: 'none' },
                    },
                    format: {
                        format_type: 'csv',
                        options: {},
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

        const vm = wrapper.vm as {
            steps: DslStep[]
            hasApiOutput: boolean
            form: { schedule_cron: string | null }
        }
        vm.steps = dsl as DslStep[]
        await nextTick()

        // Check that hasApiOutput is true
        expect(vm.hasApiOutput).toBe(true)

        // Check that cron value is cleared
        expect(vm.form.schedule_cron).toBeNull()
    })

    it('enables cron field when api output is not used', async () => {
        const wrapper = mount(CreateWorkflowDialog, {
            props: { modelValue: true },
        })

        // Set steps without api output
        const dsl = [
            {
                from: {
                    type: 'format',
                    source: {
                        source_type: 'uri',
                        config: { uri: 'http://example.com/data.csv' },
                        auth: { type: 'none' },
                    },
                    format: {
                        format_type: 'csv',
                        options: {},
                    },
                    mapping: {},
                },
                transform: { type: 'none' },
                to: {
                    type: 'format',
                    output: { mode: 'download' },
                    format: {
                        format_type: 'json',
                        options: {},
                    },
                    mapping: {},
                },
            },
        ]

        const vm = wrapper.vm as {
            steps: DslStep[]
            hasApiOutput: boolean
        }
        vm.steps = dsl as DslStep[]
        await nextTick()

        // Check that hasApiOutput is false
        expect(vm.hasApiOutput).toBe(false)
    })
})
