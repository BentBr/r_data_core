import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import DslStepEditor from './DslStepEditor.vue'
import type { DslStep } from './dsl-utils'

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k }),
}))

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getEntityFields: vi.fn().mockResolvedValue([]),
    },
}))

vi.mock('@/composables/useEntityDefinitions', () => ({
    useEntityDefinitions: () => ({
        entityDefinitions: { value: [] },
        loadEntityDefinitions: vi.fn().mockResolvedValue(undefined),
    }),
}))

describe('DslStepEditor', () => {
    beforeEach(() => {
        vi.clearAllMocks()
    })

    const defaultStep: DslStep = {
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
    }

    it('renders all three editors (from, transform, to)', async () => {
        const wrapper = mount(DslStepEditor, {
            props: {
                modelValue: defaultStep,
            },
        })

        await nextTick()

        const fromEditor = wrapper.findComponent({ name: 'DslFromEditor' })
        const transformEditor = wrapper.findComponent({ name: 'DslTransformEditor' })
        const toEditor = wrapper.findComponent({ name: 'DslToEditor' })

        expect(fromEditor.exists()).toBe(true)
        expect(transformEditor.exists()).toBe(true)
        expect(toEditor.exists()).toBe(true)
    })

    it('passes isLastStep prop to DslToEditor when provided', async () => {
        const wrapper = mount(DslStepEditor, {
            props: {
                modelValue: defaultStep,
                isLastStep: true,
            },
        })

        await nextTick()

        const toEditor = wrapper.findComponent({ name: 'DslToEditor' })
        expect(toEditor.props('isLastStep')).toBe(true)
    })

    it('passes isLastStep=false to DslToEditor when not last step', async () => {
        const wrapper = mount(DslStepEditor, {
            props: {
                modelValue: defaultStep,
                isLastStep: false,
            },
        })

        await nextTick()

        const toEditor = wrapper.findComponent({ name: 'DslToEditor' })
        expect(toEditor.props('isLastStep')).toBe(false)
    })

    it('defaults isLastStep to false when not provided', async () => {
        const wrapper = mount(DslStepEditor, {
            props: {
                modelValue: defaultStep,
            },
        })

        await nextTick()

        const toEditor = wrapper.findComponent({ name: 'DslToEditor' })
        expect(toEditor.props('isLastStep')).toBe(false)
    })

    it('passes stepIndex to DslFromEditor when provided', async () => {
        const wrapper = mount(DslStepEditor, {
            props: {
                modelValue: defaultStep,
                stepIndex: 2,
            },
        })

        await nextTick()

        const fromEditor = wrapper.findComponent({ name: 'DslFromEditor' })
        expect(fromEditor.props('stepIndex')).toBe(2)
    })

    it('passes normalizedFields to DslTransformEditor when provided', async () => {
        const normalizedFields = ['field1', 'field2', 'field3']
        const wrapper = mount(DslStepEditor, {
            props: {
                modelValue: defaultStep,
                normalizedFields,
            },
        })

        await nextTick()

        const transformEditor = wrapper.findComponent({ name: 'DslTransformEditor' })
        expect(transformEditor.props('availableFields')).toEqual(normalizedFields)
    })

    it('emits update when from is changed', async () => {
        const wrapper = mount(DslStepEditor, {
            props: {
                modelValue: defaultStep,
            },
        })

        await nextTick()

        const fromEditor = wrapper.findComponent({ name: 'DslFromEditor' })
        const newFrom = {
            type: 'format' as const,
            source: {
                source_type: 'uri' as const,
                config: { uri: 'http://example.com' },
                auth: { type: 'none' as const },
            },
            format: {
                format_type: 'csv' as const,
                options: {},
            },
            mapping: { col1: 'field1' },
        }

        await fromEditor.vm.$emit('update:modelValue', newFrom)
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as Array<[DslStep]> | undefined
        expect(emitted?.length).toBeGreaterThan(0)
        const updated = emitted![emitted!.length - 1][0]
        expect(updated.from).toEqual(newFrom)
        expect(updated.transform).toEqual(defaultStep.transform)
        expect(updated.to).toEqual(defaultStep.to)
    })

    it('emits update when transform is changed', async () => {
        const wrapper = mount(DslStepEditor, {
            props: {
                modelValue: defaultStep,
            },
        })

        await nextTick()

        const transformEditor = wrapper.findComponent({ name: 'DslTransformEditor' })
        const newTransform = {
            type: 'arithmetic' as const,
            target: 'total',
            left: { kind: 'field' as const, field: 'price' },
            op: 'mul' as const,
            right: { kind: 'const' as const, value: 1.19 },
        }

        await transformEditor.vm.$emit('update:modelValue', newTransform)
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as Array<[DslStep]> | undefined
        expect(emitted?.length).toBeGreaterThan(0)
        const updated = emitted![emitted!.length - 1][0]
        expect(updated.transform).toEqual(newTransform)
        expect(updated.from).toEqual(defaultStep.from)
        expect(updated.to).toEqual(defaultStep.to)
    })

    it('emits update when to is changed', async () => {
        const wrapper = mount(DslStepEditor, {
            props: {
                modelValue: defaultStep,
            },
        })

        await nextTick()

        const toEditor = wrapper.findComponent({ name: 'DslToEditor' })
        const newTo = {
            type: 'next_step' as const,
            mapping: { field1: 'field2' },
        }

        await toEditor.vm.$emit('update:modelValue', newTo)
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as Array<[DslStep]> | undefined
        expect(emitted?.length).toBeGreaterThan(0)
        const updated = emitted![emitted!.length - 1][0]
        expect(updated.to).toEqual(newTo)
        expect(updated.from).toEqual(defaultStep.from)
        expect(updated.transform).toEqual(defaultStep.transform)
    })
})
