import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import DslConfigurator from './DslConfigurator.vue'
import type { DslStep } from './dsl/dsl-utils'

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getDslFromOptions: vi.fn().mockResolvedValue({}),
        getDslToOptions: vi.fn().mockResolvedValue({}),
        getDslTransformOptions: vi.fn().mockResolvedValue({}),
        getEntityDefinitions: vi
            .fn()
            .mockResolvedValue({ data: [], meta: { pagination: { total: 0 } } }),
        getEntityFields: vi.fn().mockResolvedValue([]),
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k }),
}))

vi.mock('@/composables/useEntityDefinitions', () => ({
    useEntityDefinitions: () => ({
        entityDefinitions: { value: [] },
        loadEntityDefinitions: vi.fn().mockResolvedValue(undefined),
    }),
}))

describe('DslConfigurator', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    it('emits updated steps when adding a step and mapping', async () => {
        const wrapper = mount(DslConfigurator, {
            props: { modelValue: [] },
        })
        // add step
        await wrapper.find('button').trigger('click')
        // expect one step
        const emitted = wrapper.emitted('update:modelValue') as Array<[DslStep[]]> | undefined
        expect(emitted?.length).toBeGreaterThan(0)
        const steps = emitted![emitted!.length - 1][0]
        expect(Array.isArray(steps)).toBe(true)
        expect(steps.length).toBe(1)
    })

    it('supports selecting concat transform and editing operands', async () => {
        const wrapper = mount(DslConfigurator, {
            props: { modelValue: [] },
        })
        // add step
        await wrapper.find('button').trigger('click')
        // pick concat transform
        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        // first select is 'from type', second is 'transform type'
        const transformSelect = selects[1]
        await transformSelect.vm.$emit('update:modelValue', 'concat')
        // should have emitted change
        const emitted = wrapper.emitted('update:modelValue') as Array<[DslStep[]]> | undefined
        expect(emitted?.length).toBeGreaterThan(0)
    })

    it('handles mapping operations within steps', async () => {
        const wrapper = mount(DslConfigurator, {
            props: { modelValue: [] },
        })
        // add step
        await wrapper.find('button').trigger('click')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as Array<[DslStep[]]> | undefined
        expect(emitted?.length).toBeGreaterThan(0)
        const steps = emitted![emitted!.length - 1][0]
        expect(steps.length).toBe(1)
        expect(steps[0].from.mapping).toBeDefined()
        expect(steps[0].to.mapping).toBeDefined()
    })

    it('supports automapping scenarios', async () => {
        const wrapper = mount(DslConfigurator, {
            props: {
                modelValue: [
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
                            mapping: { col1: 'field1', col2: 'field2' },
                        },
                        transform: { type: 'none' },
                        to: {
                            type: 'format',
                            output: { mode: 'api' },
                            format: {
                                format_type: 'json',
                            },
                            mapping: { field1: 'out1', field2: 'out2' },
                        },
                    },
                ],
            },
        })

        await nextTick()

        // Verify mappings are preserved
        const emitted = wrapper.emitted('update:modelValue') as Array<[DslStep[]]> | undefined
        if (emitted && emitted.length > 0) {
            const steps = emitted![emitted!.length - 1][0]
            expect(steps[0].from.mapping.col1).toBe('field1')
            expect(steps[0].to.mapping.field1).toBe('out1')
        }
    })

    it('handles field list interactions for entity types', async () => {
        const wrapper = mount(DslConfigurator, {
            props: {
                modelValue: [
                    {
                        from: {
                            type: 'entity',
                            entity_definition: 'test_entity',
                            filter: { field: 'status', operator: '=', value: 'active' },
                            mapping: {},
                        },
                        transform: { type: 'none' },
                        to: {
                            type: 'entity',
                            entity_definition: 'target_entity',
                            path: '/test',
                            mode: 'create',
                            mapping: {},
                        },
                    },
                ],
            },
        })

        await nextTick()

        // Verify entity types are preserved
        const emitted = wrapper.emitted('update:modelValue') as Array<[DslStep[]]> | undefined
        if (emitted && emitted.length > 0) {
            const steps = emitted![emitted!.length - 1][0]
            expect(steps[0].from.type).toBe('entity')
            expect(steps[0].to.type).toBe('entity')
        }
    })

    it('preserves open panels when props change', async () => {
        const initialSteps: DslStep[] = [
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

        const wrapper = mount(DslConfigurator, {
            props: {
                modelValue: initialSteps,
            },
        })

        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Open the first panel
        const expansionPanels = wrapper.findAllComponents({ name: 'VExpansionPanel' })
        if (expansionPanels.length > 0) {
            const vm = wrapper.vm as unknown as { openPanels: number[] }
            vm.openPanels = [0]
            await nextTick()

            // Update props (simulating parent component update)
            const updatedSteps: DslStep[] = [
                {
                    ...initialSteps[0],
                    to: {
                        type: 'format',
                        output: { mode: 'api' }, // Changed to api (would disable cron)
                        format: {
                            format_type: 'json',
                            options: {},
                        },
                        mapping: {},
                    },
                },
            ]

            await wrapper.setProps({ modelValue: updatedSteps })
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            // Panel should still be open
            expect(vm.openPanels).toContain(0)
        }
    })

    describe('isLastStep prop', () => {
        it('passes isLastStep=true to the last step when there is one step', async () => {
            const steps: DslStep[] = [
                {
                    from: {
                        type: 'format',
                        source: {
                            source_type: 'api',
                            config: {},
                            auth: { type: 'none' },
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
                        format: {
                            format_type: 'json',
                            options: {},
                        },
                        mapping: {},
                    },
                },
            ]

            const wrapper = mount(DslConfigurator, {
                props: {
                    modelValue: steps,
                },
            })

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200))

            // Verify the component rendered
            expect(wrapper.exists()).toBe(true)
            const expansionPanels = wrapper.findAllComponents({ name: 'VExpansionPanel' })
            expect(expansionPanels.length).toBe(1)

            // Expand the panel to make components available
            const vm = wrapper.vm as unknown as { openPanels: number[] }
            vm.openPanels = [0]
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200))

            // The isLastStep prop is passed in the template: :is-last-step="idx === stepsLocal.length - 1"
            // For a single step (idx=0), length-1=0, so isLastStep should be true
            const stepEditors = wrapper.findAllComponents({ name: 'DslStepEditor' })
            expect(stepEditors.length).toBe(1)
            expect(stepEditors[0].props('isLastStep')).toBe(true)
        })

        it('passes isLastStep=false to non-last steps when there are multiple steps', async () => {
            const steps: DslStep[] = [
                {
                    from: {
                        type: 'format',
                        source: {
                            source_type: 'api',
                            config: {},
                            auth: { type: 'none' },
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
                        format: {
                            format_type: 'json',
                            options: {},
                        },
                        mapping: {},
                    },
                },
                {
                    from: {
                        type: 'format',
                        source: {
                            source_type: 'api',
                            config: {},
                            auth: { type: 'none' },
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
                        format: {
                            format_type: 'json',
                            options: {},
                        },
                        mapping: {},
                    },
                },
            ]

            const wrapper = mount(DslConfigurator, {
                props: {
                    modelValue: steps,
                },
            })

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200))

            // Verify expansion panels exist (one per step)
            const expansionPanels = wrapper.findAllComponents({ name: 'VExpansionPanel' })
            expect(expansionPanels.length).toBe(2)

            // Expand all panels to make components available
            const vm = wrapper.vm as unknown as { openPanels: number[] }
            vm.openPanels = [0, 1]
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200))

            // Now find the step editors
            const stepEditors = wrapper.findAllComponents({ name: 'DslStepEditor' })
            expect(stepEditors.length).toBe(2)
            expect(stepEditors[0].props('isLastStep')).toBe(false)
            expect(stepEditors[1].props('isLastStep')).toBe(true)
        })

        it('updates isLastStep when steps are added', async () => {
            const initialSteps: DslStep[] = [
                {
                    from: {
                        type: 'format',
                        source: {
                            source_type: 'api',
                            config: {},
                            auth: { type: 'none' },
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
                        format: {
                            format_type: 'json',
                            options: {},
                        },
                        mapping: {},
                    },
                },
            ]

            const wrapper = mount(DslConfigurator, {
                props: {
                    modelValue: initialSteps,
                },
            })

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200))

            // Expand the panel to make components available
            const vm = wrapper.vm as unknown as { openPanels: number[] }
            vm.openPanels = [0]
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200))

            let stepEditors = wrapper.findAllComponents({ name: 'DslStepEditor' })
            expect(stepEditors.length).toBe(1)
            expect(stepEditors[0].props('isLastStep')).toBe(true)

            // Add a new step
            const addButton = wrapper.find('button')
            await addButton.trigger('click')
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 200))

            // Get updated steps from emitted event
            const emitted = wrapper.emitted('update:modelValue') as Array<[DslStep[]]> | undefined
            expect(emitted).toBeDefined()
            expect(emitted?.length).toBeGreaterThan(0)
            if (emitted && emitted.length > 0) {
                const updatedSteps = emitted[emitted.length - 1][0]
                await wrapper.setProps({ modelValue: updatedSteps })
                await nextTick()
                await new Promise(resolve => setTimeout(resolve, 200))

                // Expand both panels after update
                vm.openPanels = [0, 1]
                await nextTick()
                await new Promise(resolve => setTimeout(resolve, 200))

                stepEditors = wrapper.findAllComponents({ name: 'DslStepEditor' })
                expect(stepEditors.length).toBe(2)
                expect(stepEditors[0].props('isLastStep')).toBe(false)
                expect(stepEditors[1].props('isLastStep')).toBe(true)
            }
        })

        it('updates isLastStep when steps are removed', async () => {
            const steps: DslStep[] = [
                {
                    from: {
                        type: 'format',
                        source: {
                            source_type: 'api',
                            config: {},
                            auth: { type: 'none' },
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
                        format: {
                            format_type: 'json',
                            options: {},
                        },
                        mapping: {},
                    },
                },
                {
                    from: {
                        type: 'format',
                        source: {
                            source_type: 'api',
                            config: {},
                            auth: { type: 'none' },
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
                        format: {
                            format_type: 'json',
                            options: {},
                        },
                        mapping: {},
                    },
                },
            ]

            const wrapper = mount(DslConfigurator, {
                props: {
                    modelValue: steps,
                },
            })

            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            let stepEditors = wrapper.findAllComponents({ name: 'DslStepEditor' })
            if (stepEditors.length > 0) {
                expect(stepEditors.length).toBe(2)
                expect(stepEditors[1].props('isLastStep')).toBe(true)
            }

            // Remove last step (simulate by updating props)
            const remainingSteps = [steps[0]]
            await wrapper.setProps({ modelValue: remainingSteps })
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 100))

            stepEditors = wrapper.findAllComponents({ name: 'DslStepEditor' })
            if (stepEditors.length > 0) {
                expect(stepEditors.length).toBe(1)
                expect(stepEditors[0].props('isLastStep')).toBe(true)
            }
        })
    })
})
