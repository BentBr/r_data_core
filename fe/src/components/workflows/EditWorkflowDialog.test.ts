import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import EditWorkflowDialog from './EditWorkflowDialog.vue'
import type { DslStep } from './dsl/dsl-utils'

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getWorkflow: vi.fn().mockResolvedValue({
            name: 'Test Workflow',
            description: 'Test Description',
            kind: 'consumer',
            enabled: true,
            schedule_cron: '*/5 * * * *',
            config: {
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
                ],
            },
            versioning_disabled: false,
        }),
        updateWorkflow: vi.fn().mockResolvedValue({}),
        validateDsl: vi.fn().mockResolvedValue({ ok: true }),
        listWorkflowVersions: vi.fn().mockResolvedValue([]),
        getWorkflowVersion: vi.fn().mockResolvedValue({ data: {} }),
        previewCron: vi.fn().mockResolvedValue(['2025-01-01T00:00:00Z']),
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

describe('EditWorkflowDialog', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    it('loads workflow details on open', async () => {
        const wrapper = mount(EditWorkflowDialog, {
            props: { modelValue: false, workflowUuid: 'test-uuid' },
        })

        // Open dialog
        await wrapper.setProps({ modelValue: true })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        const vm = wrapper.vm as unknown as {
            form: { name: string; description: string }
            steps: DslStep[]
            configJson: string
        }

        expect(vm.form.name).toBe('Test Workflow')
        expect(vm.form.description).toBe('Test Description')
        expect(vm.steps.length).toBe(1)
        expect(vm.configJson).toContain('"steps"')
    })

    describe('Bidirectional sync between steps and JSON', () => {
        it('syncs steps to JSON when steps change (fields → JSON)', async () => {
            const wrapper = mount(EditWorkflowDialog, {
                props: { modelValue: true, workflowUuid: 'test-uuid' },
            })

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200)) // Wait for loadDetails

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            const initialJson = vm.configJson

            // Modify steps
            const newStep = {
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
            } as unknown as DslStep

            vm.steps = [newStep]
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // JSON should be updated
            const updatedJson = vm.configJson
            expect(updatedJson).not.toBe(initialJson)
            const parsed = JSON.parse(updatedJson)
            expect(parsed.steps).toHaveLength(1)
            expect(parsed.steps[0].from.source.source_type).toBe('api')
        })

        it('syncs JSON to steps when JSON changes manually (JSON → fields)', async () => {
            const wrapper = mount(EditWorkflowDialog, {
                props: { modelValue: true, workflowUuid: 'test-uuid' },
            })

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200)) // Wait for loadDetails

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            // Manually edit JSON
            const newJson = JSON.stringify(
                {
                    steps: [
                        {
                            from: {
                                type: 'format',
                                source: {
                                    source_type: 'uri',
                                    config: { uri: 'http://manually-edited.com/data.csv' },
                                },
                                format: {
                                    format_type: 'csv',
                                    options: { has_header: false },
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
            await new Promise(resolve => setTimeout(resolve, 100))

            // Steps should be updated
            expect(vm.steps.length).toBe(1)
            const step = vm.steps[0]
            if (
                step.from &&
                'type' in step.from &&
                step.from.type === 'format' &&
                'source' in step.from
            ) {
                const formatFrom = step.from as {
                    type: 'format'
                    source: { config: { uri?: string } }
                }
                expect(formatFrom.source.config.uri).toBe('http://manually-edited.com/data.csv')
            } else {
                throw new Error('Expected format from type with source')
            }
        })

        it('prevents circular updates when syncing steps → JSON → steps', async () => {
            const wrapper = mount(EditWorkflowDialog, {
                props: { modelValue: true, workflowUuid: 'test-uuid' },
            })

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200)) // Wait for loadDetails

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            // Get initial state
            const initialSteps = [...vm.steps]

            // Set steps (should trigger steps → JSON)
            vm.steps = initialSteps
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            const jsonAfterSteps = vm.configJson

            // Set JSON back to same value (should NOT trigger JSON → steps due to circular prevention)
            vm.configJson = jsonAfterSteps
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Steps should remain the same (no circular update)
            expect(vm.steps).toEqual(initialSteps)
        })

        it('handles invalid JSON gracefully without breaking steps', async () => {
            const wrapper = mount(EditWorkflowDialog, {
                props: { modelValue: true, workflowUuid: 'test-uuid' },
            })

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200)) // Wait for loadDetails

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            const stepsBeforeInvalidJson = [...vm.steps]

            // Set invalid JSON
            vm.configJson = '{"steps": [invalid json}'
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Steps should remain unchanged
            expect(vm.steps).toEqual(stepsBeforeInvalidJson)
        })

        it('preserves manual JSON edits on save', async () => {
            const wrapper = mount(EditWorkflowDialog, {
                props: { modelValue: true, workflowUuid: 'test-uuid' },
            })

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200)) // Wait for loadDetails

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
                submit: () => Promise<void>
            }

            // Manually edit JSON
            const manuallyEditedJson = JSON.stringify(
                {
                    steps: [
                        {
                            from: {
                                type: 'format',
                                source: {
                                    source_type: 'uri',
                                    config: { uri: 'http://manually-edited.com/data.csv' },
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
                    ],
                },
                null,
                2
            )

            vm.configJson = manuallyEditedJson
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Submit should use the manually edited JSON
            await vm.submit()

            // Verify that updateWorkflow was called with the manually edited JSON
            const { typedHttpClient } = await import('@/api/typed-client')
            expect(typedHttpClient.updateWorkflow).toHaveBeenCalled()
            const callArgs = (typedHttpClient.updateWorkflow as ReturnType<typeof vi.fn>).mock
                .calls[0][1]
            expect(JSON.stringify(callArgs.config)).toContain('manually-edited.com')
        })

        it('sanitizes steps when loading from JSON', async () => {
            const wrapper = mount(EditWorkflowDialog, {
                props: { modelValue: true, workflowUuid: 'test-uuid' },
            })

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200)) // Wait for loadDetails

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

            // Steps should be sanitized
            expect(vm.steps.length).toBe(1)
            const step = vm.steps[0]
            if (step.from && 'type' in step.from && step.from.type === 'format') {
                const formatFrom = step.from as {
                    type: 'format'
                    format: { format_type: string; options?: unknown }
                }
                if (formatFrom.format?.format_type === 'csv') {
                    // CSV options should be ensured by sanitization
                    expect(formatFrom.format.options).toBeDefined()
                }
            }
        })

        it('handles empty JSON without breaking', async () => {
            const wrapper = mount(EditWorkflowDialog, {
                props: { modelValue: true, workflowUuid: 'test-uuid' },
            })

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200)) // Wait for loadDetails

            const vm = wrapper.vm as {
                steps: DslStep[]
                configJson: string
            }

            const stepsBeforeEmpty = [...vm.steps]

            // Set empty JSON
            vm.configJson = ''
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Should not throw error, steps should remain as they were
            expect(vm.steps).toBeDefined()
            expect(vm.steps).toEqual(stepsBeforeEmpty)
        })

        it('handles JSON with missing steps array', async () => {
            const wrapper = mount(EditWorkflowDialog, {
                props: { modelValue: true, workflowUuid: 'test-uuid' },
            })

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200)) // Wait for loadDetails

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

        it('validates JSON before syncing to steps', async () => {
            const wrapper = mount(EditWorkflowDialog, {
                props: { modelValue: true, workflowUuid: 'test-uuid' },
            })

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200)) // Wait for loadDetails

            const vm = wrapper.vm as unknown as {
                steps: DslStep[]
                configJson: string
                configError: string | null
                submit: () => Promise<void>
            }

            const initialSteps = [...vm.steps]

            // Set invalid JSON
            vm.configJson = '{"steps": [malformed}'
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Steps should remain unchanged
            expect(vm.steps).toEqual(initialSteps)

            // Try to submit with invalid JSON
            await vm.submit()

            // Should have error
            expect(vm.configError).toBeTruthy()
            expect(vm.configError).toContain('Invalid JSON')
        })
    })
})
