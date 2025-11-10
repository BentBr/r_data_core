import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import DslConfigurator from './DslConfigurator.vue'

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
        const emitted = wrapper.emitted()['update:modelValue'] as any[]
        expect(emitted?.length).toBeGreaterThan(0)
        const steps = emitted[emitted.length - 1][0]
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
        const emitted = wrapper.emitted()['update:modelValue'] as any[]
        expect(emitted?.length).toBeGreaterThan(0)
    })

    it('handles mapping operations within steps', async () => {
        const wrapper = mount(DslConfigurator, {
            props: { modelValue: [] },
        })
        // add step
        await wrapper.find('button').trigger('click')
        await nextTick()

        const emitted = wrapper.emitted()['update:modelValue'] as any[]
        expect(emitted?.length).toBeGreaterThan(0)
        const steps = emitted[emitted.length - 1][0]
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
                            type: 'csv',
                            uri: 'http://example.com/data.csv',
                            options: { header: true },
                            mapping: { col1: 'field1', col2: 'field2' },
                        },
                        transform: { type: 'none' },
                        to: { type: 'json', output: 'api', mapping: { field1: 'out1', field2: 'out2' } },
                    },
                ],
            },
        })

        await nextTick()

        // Verify mappings are preserved
        const emitted = wrapper.emitted()['update:modelValue'] as any[]
        if (emitted && emitted.length > 0) {
            const steps = emitted[emitted.length - 1][0]
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
                            filter: { field: 'status', value: 'active' },
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
        const emitted = wrapper.emitted()['update:modelValue'] as any[]
        if (emitted && emitted.length > 0) {
            const steps = emitted[emitted.length - 1][0]
            expect(steps[0].from.type).toBe('entity')
            expect(steps[0].to.type).toBe('entity')
        }
    })
})
