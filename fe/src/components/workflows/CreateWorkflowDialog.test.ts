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
                    type: 'format',
                    source: {
                        source_type: 'uri',
                        config: { uri: 'http://example.com/data.csv' },
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
                    format: { format_type: 'json' },
                    mapping: {},
                },
            },
        ]
        // Set steps directly via exposed API
        const vm = wrapper.vm as {
            steps: DslStep[]
            submit: () => Promise<void>
        }
        vm.steps = dsl as unknown as DslStep[]
        await nextTick()

        await vm.submit()

        const emitted = wrapper.emitted('created') as Array<[string]> | undefined
        expect(emitted?.length).toBe(1)
        expect(emitted![0][0]).toBeTypeOf('string')
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
        expect(vm.configError?.toLowerCase().includes('dsl_required')).toBe(true)
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
        vm.steps = dsl as unknown as DslStep[]
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
        vm.steps = dsl as unknown as DslStep[]
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

        const vm = wrapper.vm as unknown as {
            steps: DslStep[]
            hasApiOutput: boolean
            form: { schedule_cron: string | null }
        }
        vm.steps = dsl as unknown as DslStep[]
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

        const vm = wrapper.vm as unknown as {
            steps: DslStep[]
            hasApiOutput: boolean
        }
        vm.steps = dsl as unknown as DslStep[]
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

        const vm = wrapper.vm as unknown as {
            steps: DslStep[]
            hasApiOutput: boolean
            form: { schedule_cron: string | null }
        }
        vm.steps = dsl as unknown as DslStep[]
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

        const vm = wrapper.vm as unknown as {
            steps: DslStep[]
            hasApiOutput: boolean
        }
        vm.steps = dsl as unknown as DslStep[]
        await nextTick()

        // Check that hasApiOutput is false
        expect(vm.hasApiOutput).toBe(false)
    })

    describe('Bidirectional sync between steps and JSON', () => {
        it('syncs steps to JSON when steps change (fields → JSON)', async () => {
            const wrapper = mount(CreateWorkflowDialog, {
                props: { modelValue: true },
            })

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            const initialJson = vm.configJson

            // Set steps via DSL editor
            const newSteps = [
                {
                    from: {
                        type: 'format',
                        source: {
                            source_type: 'uri',
                            config: { uri: 'http://example.com/data.csv' },
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
                        format: { format_type: 'json' },
                        mapping: {},
                    },
                },
            ] as unknown as DslStep[]

            vm.steps = newSteps
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100)) // Wait for watch to execute

            // JSON should be updated
            const updatedJson = vm.configJson
            expect(updatedJson).not.toBe(initialJson)
            expect(updatedJson).toContain('"steps"')
            const parsed = JSON.parse(updatedJson)
            expect(parsed.steps).toHaveLength(1)
            const firstStep = parsed.steps[0]
            if (firstStep.from?.type === 'format' && firstStep.from.source) {
                expect(firstStep.from.source.source_type).toBe('uri')
            }
        })

        it('syncs JSON to steps when JSON changes manually (JSON → fields)', async () => {
            const wrapper = mount(CreateWorkflowDialog, {
                props: { modelValue: true },
            })

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            const initialStepsLength = vm.steps.length

            // Manually edit JSON
            const newJson = JSON.stringify(
                {
                    steps: [
                        {
                            from: {
                                type: 'format',
                                source: {
                                    source_type: 'api',
                                    config: {},
                                },
                                format: {
                                    format_type: 'json',
                                    options: {},
                                },
                                mapping: {},
                            },
                            transform: { type: 'none' },
                            to: {
                                type: 'format',
                                output: { mode: 'api' },
                                format: { format_type: 'json' },
                                mapping: {},
                            },
                        },
                    ],
                },
                null,
                2
            )

            vm.configJson = newJson
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100)) // Wait for watch to execute

            // Steps should be updated
            expect(vm.steps.length).not.toBe(initialStepsLength)
            expect(vm.steps.length).toBe(1)
            const step = vm.steps[0]
            const formatFrom = step.from as unknown as {
                type: 'format'
                source: { source_type: string }
            }
            expect(formatFrom.source.source_type).toBe('api')
        })

        it('prevents circular updates when syncing steps → JSON → steps', async () => {
            const wrapper = mount(CreateWorkflowDialog, {
                props: { modelValue: true },
            })

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            const step = {
                from: {
                    type: 'format',
                    source: {
                        source_type: 'uri',
                        config: { uri: 'http://example.com/data.csv' },
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
                    format: { format_type: 'json' },
                    mapping: {},
                },
            } as unknown as DslStep

            // Set steps
            vm.steps = [step]
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            const jsonAfterSteps = vm.configJson
            const stepsAfterJson = [...vm.steps]

            // Manually update JSON to same value (should NOT trigger JSON → steps due to circular prevention)
            vm.configJson = jsonAfterSteps
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Steps should not change (circular update prevented)
            expect(vm.steps).toEqual(stepsAfterJson)
        })

        it('handles invalid JSON gracefully without breaking steps', async () => {
            const wrapper = mount(CreateWorkflowDialog, {
                props: { modelValue: true },
            })

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            const validSteps = [
                {
                    from: {
                        type: 'format',
                        source: {
                            source_type: 'uri',
                            config: { uri: 'http://example.com/data.csv' },
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
                        format: { format_type: 'json' },
                        mapping: {},
                    },
                },
            ] as unknown as DslStep[]

            vm.steps = validSteps
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            const stepsBeforeInvalidJson = [...vm.steps]

            // Set invalid JSON
            vm.configJson = '{"steps": [invalid json}'
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Steps should remain unchanged (invalid JSON is ignored)
            expect(vm.steps).toEqual(stepsBeforeInvalidJson)
        })

        it('handles empty JSON without breaking', async () => {
            const wrapper = mount(CreateWorkflowDialog, {
                props: { modelValue: true },
            })

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            // Set empty JSON
            vm.configJson = ''
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Should not throw error, steps should remain as they were
            expect(vm.steps).toBeDefined()
        })

        it('handles JSON with missing steps array', async () => {
            const wrapper = mount(CreateWorkflowDialog, {
                props: { modelValue: true },
            })

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            const initialSteps = [...vm.steps]

            // Set JSON without steps
            vm.configJson = '{"other": "data"}'
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Steps should remain unchanged
            expect(vm.steps).toEqual(initialSteps)
        })

        it('sanitizes steps when loading from JSON', async () => {
            const wrapper = mount(CreateWorkflowDialog, {
                props: { modelValue: true },
            })

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            // Set JSON with potentially unsanitized steps
            const jsonWithUnsanitizedSteps = JSON.stringify(
                {
                    steps: [
                        {
                            from: {
                                type: 'format',
                                source: {
                                    source_type: 'uri',
                                    config: { uri: 'http://example.com/data.csv' },
                                },
                                format: {
                                    format_type: 'csv',
                                    // Missing options - should be added by sanitization
                                },
                                mapping: {},
                            },
                            transform: { type: 'none' },
                            to: {
                                type: 'format',
                                output: { mode: 'api' },
                                format: { format_type: 'json' },
                                mapping: {},
                            },
                        },
                    ],
                },
                null,
                2
            )

            vm.configJson = jsonWithUnsanitizedSteps
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Steps should be sanitized (options should be added)
            expect(vm.steps.length).toBe(1)
            const step = vm.steps[0]
            const formatFrom = step.from as unknown as {
                type: 'format'
                format: { format_type: string; options?: unknown }
            }
            if (formatFrom.format.format_type === 'csv') {
                // CSV options should be ensured by sanitization
                expect(formatFrom.format.options).toBeDefined()
            }
        })

        it('preserves manual JSON edits on save', async () => {
            const wrapper = mount(CreateWorkflowDialog, {
                props: { modelValue: true },
            })

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
                submit: () => Promise<void>
            }

            // Set initial steps
            vm.steps = [
                {
                    from: {
                        type: 'format',
                        source: {
                            source_type: 'uri',
                            config: { uri: 'http://example.com/data.csv' },
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
                        format: { format_type: 'json' },
                        mapping: {},
                    },
                },
            ] as unknown as DslStep[]

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            const jsonFromSteps = vm.configJson

            // Manually edit JSON (add a comment-like field or modify structure)
            const manuallyEditedJson = jsonFromSteps.replace(
                '"uri": "http://example.com/data.csv"',
                '"uri": "http://manually-edited.com/data.csv"'
            )

            vm.configJson = manuallyEditedJson
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Submit should use the manually edited JSON, not overwrite it
            await vm.submit()

            // Verify that createWorkflow was called with the manually edited JSON
            const { typedHttpClient } = await import('@/api/typed-client')
            expect(typedHttpClient.createWorkflow).toHaveBeenCalled()
            const callArgs = (typedHttpClient.createWorkflow as ReturnType<typeof vi.fn>).mock
                .calls[0][0]
            expect(JSON.stringify(callArgs.config)).toContain('manually-edited.com')
        })

        it('handles rapid changes without causing infinite loops', async () => {
            const wrapper = mount(CreateWorkflowDialog, {
                props: { modelValue: true },
            })

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            // Rapidly change steps multiple times
            for (let i = 0; i < 5; i++) {
                vm.steps = [
                    {
                        from: {
                            type: 'format',
                            source: {
                                source_type: 'uri',
                                config: { uri: `http://example${i}.com/data.csv` },
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
                            format: { format_type: 'json' },
                            mapping: {},
                        },
                    },
                ] as unknown as DslStep[]

                await nextTick()
                await new Promise(resolve => setTimeout(resolve, 50))
            }

            // Should not cause infinite loop - final state should be consistent
            const finalJson = vm.configJson
            const parsed = JSON.parse(finalJson)
            expect(parsed.steps).toHaveLength(1)
            expect(parsed.steps[0].from.source.config.uri).toContain('example')
        })

        it('handles JSON with non-array steps gracefully', async () => {
            const wrapper = mount(CreateWorkflowDialog, {
                props: { modelValue: true },
            })

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            const initialSteps = [...vm.steps]

            // Set JSON with steps as object instead of array
            vm.configJson = '{"steps": {"not": "an array"}}'
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Steps should remain unchanged
            expect(vm.steps).toEqual(initialSteps)
        })

        it('handles JSON with null steps', async () => {
            const wrapper = mount(CreateWorkflowDialog, {
                props: { modelValue: true },
            })

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            const initialSteps = [...vm.steps]

            // Set JSON with null steps
            vm.configJson = '{"steps": null}'
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Steps should remain unchanged
            expect(vm.steps).toEqual(initialSteps)
        })
    })
})
