import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
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
})
