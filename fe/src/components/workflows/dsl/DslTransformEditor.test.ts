import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import DslTransformEditor from './DslTransformEditor.vue'
import type { Transform } from './contracts'

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k }),
}))

// Default mock: workflow mail not configured
const mockWorkflowMailConfigured = { value: false }

vi.mock('@/stores/capabilities', () => ({
    useCapabilitiesStore: () => ({
        get workflowMailConfigured() {
            return mockWorkflowMailConfigured.value
        },
    }),
}))

describe('DslTransformEditor', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockWorkflowMailConfigured.value = false
    })

    const defaultTransform: Transform = { type: 'none' }

    describe('BuildPath Transform', () => {
        it('renders build_path transform fields', async () => {
            const transform: Transform = {
                type: 'build_path',
                target: 'instance_path',
                template: '/statistics_instance/{license_key_id}',
                separator: '/',
            }

            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: transform,
                },
            })

            await nextTick()

            // Check that build_path specific fields are rendered
            const targetInput = wrapper.find('input[type="text"]')
            expect(targetInput.exists()).toBe(true)

            // Check template textarea exists
            const templateTextarea = wrapper.find('textarea')
            expect(templateTextarea.exists()).toBe(true)
        })

        it('updates build_path transform when fields change', async () => {
            const transform: Transform = {
                type: 'build_path',
                target: 'instance_path',
                template: '/statistics_instance/{license_key_id}',
            }

            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: transform,
                },
            })

            await nextTick()

            // Find and update template field
            const templateTextarea = wrapper.find('textarea')
            await templateTextarea.setValue('/new/path/{field}')

            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            expect(emitted).toBeTruthy()
            if (emitted?.[0]) {
                const updated = emitted[0][0] as Transform
                expect(updated.type).toBe('build_path')
                if (updated.type === 'build_path') {
                    expect(updated.template).toBe('/new/path/{field}')
                }
            }
        })
    })

    describe('ResolveEntityPath Transform', () => {
        it('renders resolve_entity_path transform fields', async () => {
            const transform: Transform = {
                type: 'resolve_entity_path',
                target_path: 'instance_path',
                entity_type: 'statistics_instance',
                filters: {
                    license_key_id: {
                        kind: 'field',
                        field: 'license_key_id',
                    },
                },
            }

            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: transform,
                },
            })

            await nextTick()

            // Check that resolve_entity_path specific fields are rendered
            const targetPathInput = wrapper.find('input[type="text"]')
            expect(targetPathInput.exists()).toBe(true)
        })

        it('allows adding filters', async () => {
            const transform: Transform = {
                type: 'resolve_entity_path',
                target_path: 'instance_path',
                entity_type: 'statistics_instance',
                filters: {},
            }

            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: transform,
                },
            })

            await nextTick()

            // Find add filter button
            const addButton = wrapper.find('button')
            expect(addButton.exists()).toBe(true)

            await addButton.trigger('click')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            expect(emitted).toBeTruthy()
            if (emitted?.[0]) {
                const updated = emitted[0][0] as Transform
                expect(updated.type).toBe('resolve_entity_path')
                if (updated.type === 'resolve_entity_path') {
                    expect(Object.keys(updated.filters).length).toBeGreaterThan(0)
                }
            }
        })
    })

    describe('GetOrCreateEntity Transform', () => {
        it('renders get_or_create_entity transform fields', async () => {
            const transform: Transform = {
                type: 'get_or_create_entity',
                target_path: 'instance_path',
                entity_type: 'statistics_instance',
                path_template: '/statistics_instance/{license_key_id}',
            }

            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: transform,
                },
            })

            await nextTick()

            // Check that get_or_create_entity specific fields are rendered
            const targetPathInput = wrapper.find('input[type="text"]')
            expect(targetPathInput.exists()).toBe(true)

            // Check path template textarea exists
            const templateTextarea = wrapper.find('textarea')
            expect(templateTextarea.exists()).toBe(true)
        })

        it('updates get_or_create_entity transform when fields change', async () => {
            const transform: Transform = {
                type: 'get_or_create_entity',
                target_path: 'instance_path',
                entity_type: 'statistics_instance',
                path_template: '/statistics_instance/{license_key_id}',
            }

            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: transform,
                },
            })

            await nextTick()

            // Find and update entity_type field
            const inputs = wrapper.findAll('input[type="text"]')
            const entityTypeInput = inputs.find(input =>
                input.attributes('label')?.includes('Entity Type')
            )

            if (entityTypeInput) {
                await entityTypeInput.setValue('new_entity_type')
                await nextTick()

                const emitted = wrapper.emitted('update:modelValue')
                expect(emitted).toBeTruthy()
            }
        })
    })

    describe('Transform Type Selection', () => {
        it('switches between transform types', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: defaultTransform,
                },
            })

            await nextTick()

            // Find the transform type select
            const select = wrapper.findComponent({ name: 'VSelect' })
            expect(select.exists()).toBe(true)

            // Change to build_path
            await select.vm.$emit('update:modelValue', 'build_path')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            expect(emitted).toBeTruthy()
            if (emitted?.[0]) {
                const updated = emitted[0][0] as Transform
                expect(updated.type).toBe('build_path')
            }
        })

        it('includes new transform types in dropdown', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: defaultTransform,
                },
            })

            await nextTick()

            const select = wrapper.findComponent({ name: 'VSelect' })
            const items = select.props('items') as Array<{ title: string; value: string }>

            const transformTypes = items.map(item => item.value)
            expect(transformTypes).toContain('build_path')
            expect(transformTypes).toContain('resolve_entity_path')
            expect(transformTypes).toContain('get_or_create_entity')
        })

        it('hides send_email option when workflowMailConfigured is false', async () => {
            mockWorkflowMailConfigured.value = false

            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: defaultTransform,
                },
            })

            await nextTick()

            const select = wrapper.findComponent({ name: 'VSelect' })
            const items = select.props('items') as Array<{ title: string; value: string }>
            const transformTypes = items.map(item => item.value)

            expect(transformTypes).not.toContain('send_email')
        })

        it('shows send_email option when workflowMailConfigured is true', async () => {
            mockWorkflowMailConfigured.value = true

            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: defaultTransform,
                },
            })

            await nextTick()

            const select = wrapper.findComponent({ name: 'VSelect' })
            const items = select.props('items') as Array<{ title: string; value: string }>
            const transformTypes = items.map(item => item.value)

            expect(transformTypes).toContain('send_email')
        })

        it('switches to authenticate and emits default authenticate transform', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: defaultTransform },
            })
            await nextTick()

            const select = wrapper.findComponent({ name: 'VSelect' })
            await select.vm.$emit('update:modelValue', 'authenticate')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            expect(updated.type).toBe('authenticate')
        })

        it('switches to send_email and emits default send_email transform', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: defaultTransform },
            })
            await nextTick()

            const select = wrapper.findComponent({ name: 'VSelect' })
            await select.vm.$emit('update:modelValue', 'send_email')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            expect(updated.type).toBe('send_email')
            if (updated.type === 'send_email') {
                expect(updated.to).toEqual([])
            }
        })

        it('switches to arithmetic and emits default arithmetic transform', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: defaultTransform },
            })
            await nextTick()

            const select = wrapper.findComponent({ name: 'VSelect' })
            await select.vm.$emit('update:modelValue', 'arithmetic')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            expect(updated.type).toBe('arithmetic')
            if (updated.type === 'arithmetic') {
                expect(updated.left).toEqual({ kind: 'field', field: '' })
                expect(updated.right).toEqual({ kind: 'const', value: 0 })
            }
        })

        it('switches to concat and emits default concat transform', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: defaultTransform },
            })
            await nextTick()

            const select = wrapper.findComponent({ name: 'VSelect' })
            await select.vm.$emit('update:modelValue', 'concat')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            expect(updated.type).toBe('concat')
            if (updated.type === 'concat') {
                expect(updated.separator).toBe(' ')
            }
        })

        it('switches to resolve_entity_path and emits default transform', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: defaultTransform },
            })
            await nextTick()

            const select = wrapper.findComponent({ name: 'VSelect' })
            await select.vm.$emit('update:modelValue', 'resolve_entity_path')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            expect(updated.type).toBe('resolve_entity_path')
        })

        it('switches to get_or_create_entity and emits default transform', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: defaultTransform },
            })
            await nextTick()

            const select = wrapper.findComponent({ name: 'VSelect' })
            await select.vm.$emit('update:modelValue', 'get_or_create_entity')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            expect(updated.type).toBe('get_or_create_entity')
        })

        it('switches to none and emits { type: none }', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: {
                        type: 'arithmetic',
                        target: 'x',
                        left: { kind: 'const', value: 1 },
                        op: 'add',
                        right: { kind: 'const', value: 2 },
                    },
                },
            })
            await nextTick()

            const select = wrapper.findComponent({ name: 'VSelect' })
            await select.vm.$emit('update:modelValue', 'none')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            expect(updated).toEqual({ type: 'none' })
        })
    })

    describe('Arithmetic Transform', () => {
        const arithmeticTransform: Transform = {
            type: 'arithmetic',
            target: 'total',
            left: { kind: 'field', field: 'a' },
            op: 'add',
            right: { kind: 'const', value: 5 },
        }

        it('renders target, op, left and right operand controls', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: arithmeticTransform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const targetField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.target'
            )
            expect(targetField).toBeTruthy()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            // transform type, op, left kind, right kind (left is field so there's also left field select)
            expect(selects.length).toBeGreaterThanOrEqual(4)
        })

        it('updates the target field', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: arithmeticTransform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const targetField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.target'
            )
            await targetField!.vm.$emit('update:modelValue', 'new_total')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'arithmetic') {
                expect(updated.target).toBe('new_total')
            }
        })

        it('updates the arithmetic op', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: arithmeticTransform },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const opSelect = selects.find(s => {
                const items = s.props('items') as Array<{ value: string }> | undefined
                return Array.isArray(items) && items.some(i => i.value === 'mul')
            })
            expect(opSelect).toBeTruthy()
            await opSelect!.vm.$emit('update:modelValue', 'mul')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'arithmetic') {
                expect(updated.op).toBe('mul')
            }
        })

        it('switches left operand kind from field to const', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: arithmeticTransform },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const leftKindSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                return label === 'workflows.dsl.left_kind'
            })
            expect(leftKindSelect).toBeTruthy()
            await leftKindSelect!.vm.$emit('update:modelValue', 'const')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'arithmetic') {
                expect(updated.left).toEqual({ kind: 'const', value: 0 })
            }
        })

        it('switches left operand kind from const to field', async () => {
            const constLeftTransform: Transform = {
                ...arithmeticTransform,
                left: { kind: 'const', value: 1 },
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: constLeftTransform },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const leftKindSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                return label === 'workflows.dsl.left_kind'
            })
            await leftKindSelect!.vm.$emit('update:modelValue', 'field')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'arithmetic') {
                expect(updated.left).toEqual({ kind: 'field', field: '' })
            }
        })

        it('switches right operand kind from field to const', async () => {
            const fieldRightTransform: Transform = {
                ...arithmeticTransform,
                right: { kind: 'field', field: 'z' },
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: fieldRightTransform },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const rightKindSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                return label === 'workflows.dsl.right_kind'
            })
            await rightKindSelect!.vm.$emit('update:modelValue', 'const')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'arithmetic') {
                expect(updated.right).toEqual({ kind: 'const', value: 0 })
            }
        })

        it('updates left field value using a text field when no availableFields', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: arithmeticTransform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const leftFieldInput = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.left_field'
            )
            expect(leftFieldInput).toBeTruthy()
            await leftFieldInput!.vm.$emit('update:modelValue', 'b')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'arithmetic') {
                expect(updated.left).toEqual({ kind: 'field', field: 'b' })
            }
        })

        it('uses a select for left field when availableFields provided', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: arithmeticTransform,
                    availableFields: ['a', 'b', 'c'],
                },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const leftFieldSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                return label === 'workflows.dsl.left_field'
            })
            expect(leftFieldSelect).toBeTruthy()
            await leftFieldSelect!.vm.$emit('update:modelValue', 'c')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'arithmetic') {
                expect(updated.left).toEqual({ kind: 'field', field: 'c' })
            }
        })

        it('updates left const value as a number', async () => {
            const constLeftTransform: Transform = {
                ...arithmeticTransform,
                left: { kind: 'const', value: 1 },
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: constLeftTransform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const leftValueInput = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.left_value'
            )
            expect(leftValueInput).toBeTruthy()
            await leftValueInput!.vm.$emit('update:modelValue', '42')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'arithmetic') {
                expect(updated.left).toEqual({ kind: 'const', value: 42 })
            }
        })

        it('switches right operand kind from const to field', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: arithmeticTransform },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const rightKindSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                return label === 'workflows.dsl.right_kind'
            })
            expect(rightKindSelect).toBeTruthy()
            await rightKindSelect!.vm.$emit('update:modelValue', 'field')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'arithmetic') {
                expect(updated.right).toEqual({ kind: 'field', field: '' })
            }
        })

        it('updates right field value', async () => {
            const rightFieldTransform: Transform = {
                ...arithmeticTransform,
                right: { kind: 'field', field: '' },
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: rightFieldTransform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const rightFieldInput = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.right_field'
            )
            expect(rightFieldInput).toBeTruthy()
            await rightFieldInput!.vm.$emit('update:modelValue', 'd')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'arithmetic') {
                expect(updated.right).toEqual({ kind: 'field', field: 'd' })
            }
        })

        it('uses a select for right field when availableFields provided', async () => {
            const rightFieldTransform: Transform = {
                ...arithmeticTransform,
                right: { kind: 'field', field: '' },
            }
            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: rightFieldTransform,
                    availableFields: ['a', 'b'],
                },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const rightFieldSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                return label === 'workflows.dsl.right_field'
            })
            expect(rightFieldSelect).toBeTruthy()
            await rightFieldSelect!.vm.$emit('update:modelValue', 'b')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'arithmetic') {
                expect(updated.right).toEqual({ kind: 'field', field: 'b' })
            }
        })

        it('updates right const value as a number', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: arithmeticTransform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const rightValueInput = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.right_value'
            )
            expect(rightValueInput).toBeTruthy()
            await rightValueInput!.vm.$emit('update:modelValue', '99')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'arithmetic') {
                expect(updated.right).toEqual({ kind: 'const', value: 99 })
            }
        })

        it('syncs local operand state when modelValue changes externally', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: arithmeticTransform },
            })
            await nextTick()

            const updatedTransform: Transform = {
                type: 'arithmetic',
                target: 'total',
                left: { kind: 'const', value: 7 },
                op: 'sub',
                right: { kind: 'field', field: 'z' },
            }
            await wrapper.setProps({ modelValue: updatedTransform })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const leftValueInput = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.left_value'
            )
            expect(leftValueInput?.props('modelValue')).toBe('7')
        })
    })

    describe('Concat Transform', () => {
        const concatTransform: Transform = {
            type: 'concat',
            target: 'full_name',
            left: { kind: 'field', field: 'first_name' },
            separator: ' ',
            right: { kind: 'field', field: 'last_name' },
        }

        it('renders target, separator, left and right operand controls', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: concatTransform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const separatorField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.separator'
            )
            expect(separatorField).toBeTruthy()
        })

        it('updates the concat target', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: concatTransform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const targetField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.target'
            )
            await targetField!.vm.$emit('update:modelValue', 'name')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'concat') {
                expect(updated.target).toBe('name')
            }
        })

        it('updates the concat separator', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: concatTransform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const separatorField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.separator'
            )
            await separatorField!.vm.$emit('update:modelValue', '-')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'concat') {
                expect(updated.separator).toBe('-')
            }
        })

        it('switches left concat kind from field to const_string', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: concatTransform },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const leftKindSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                return label === 'workflows.dsl.left_kind'
            })
            expect(leftKindSelect).toBeTruthy()
            await leftKindSelect!.vm.$emit('update:modelValue', 'const_string')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'concat') {
                expect(updated.left).toEqual({ kind: 'const_string', value: '' })
            }
        })

        it('switches left concat kind from const_string to field', async () => {
            const constLeftConcat: Transform = {
                ...concatTransform,
                left: { kind: 'const_string', value: 'x' },
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: constLeftConcat },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const leftKindSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                return label === 'workflows.dsl.left_kind'
            })
            await leftKindSelect!.vm.$emit('update:modelValue', 'field')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'concat') {
                expect(updated.left).toEqual({ kind: 'field', field: '' })
            }
        })

        it('switches right concat kind from const_string to field', async () => {
            const constRightConcat: Transform = {
                ...concatTransform,
                right: { kind: 'const_string', value: 'x' },
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: constRightConcat },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const rightKindSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                return label === 'workflows.dsl.right_kind'
            })
            await rightKindSelect!.vm.$emit('update:modelValue', 'field')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'concat') {
                expect(updated.right).toEqual({ kind: 'field', field: '' })
            }
        })

        it('updates left concat field value with text field when no availableFields', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: concatTransform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const leftFieldInput = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.left_field'
            )
            expect(leftFieldInput).toBeTruthy()
            await leftFieldInput!.vm.$emit('update:modelValue', 'given_name')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'concat') {
                expect(updated.left).toEqual({ kind: 'field', field: 'given_name' })
            }
        })

        it('uses select for left concat field when availableFields provided', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: concatTransform,
                    availableFields: ['first_name', 'last_name'],
                },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const leftFieldSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                return label === 'workflows.dsl.left_field'
            })
            expect(leftFieldSelect).toBeTruthy()
            await leftFieldSelect!.vm.$emit('update:modelValue', 'last_name')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'concat') {
                expect(updated.left).toEqual({ kind: 'field', field: 'last_name' })
            }
        })

        it('updates left concat const value', async () => {
            const constLeftConcat: Transform = {
                ...concatTransform,
                left: { kind: 'const_string', value: '' },
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: constLeftConcat },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const leftValueInput = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.left_value'
            )
            expect(leftValueInput).toBeTruthy()
            await leftValueInput!.vm.$emit('update:modelValue', 'Mr.')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'concat') {
                expect(updated.left).toEqual({ kind: 'const_string', value: 'Mr.' })
            }
        })

        it('switches right concat kind from field to const_string', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: concatTransform },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const rightKindSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                return label === 'workflows.dsl.right_kind'
            })
            expect(rightKindSelect).toBeTruthy()
            await rightKindSelect!.vm.$emit('update:modelValue', 'const_string')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'concat') {
                expect(updated.right).toEqual({ kind: 'const_string', value: '' })
            }
        })

        it('updates right concat field value', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: concatTransform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const rightFieldInput = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.right_field'
            )
            expect(rightFieldInput).toBeTruthy()
            await rightFieldInput!.vm.$emit('update:modelValue', 'surname')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'concat') {
                expect(updated.right).toEqual({ kind: 'field', field: 'surname' })
            }
        })

        it('uses select for right concat field when availableFields provided', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: {
                    modelValue: concatTransform,
                    availableFields: ['first_name', 'last_name'],
                },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const rightFieldSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                return label === 'workflows.dsl.right_field'
            })
            expect(rightFieldSelect).toBeTruthy()
            await rightFieldSelect!.vm.$emit('update:modelValue', 'first_name')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'concat') {
                expect(updated.right).toEqual({ kind: 'field', field: 'first_name' })
            }
        })

        it('updates right concat const value', async () => {
            const constRightConcat: Transform = {
                ...concatTransform,
                right: { kind: 'const_string', value: '' },
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: constRightConcat },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const rightValueInput = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.right_value'
            )
            expect(rightValueInput).toBeTruthy()
            await rightValueInput!.vm.$emit('update:modelValue', 'Jr.')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'concat') {
                expect(updated.right).toEqual({ kind: 'const_string', value: 'Jr.' })
            }
        })
    })

    describe('ResolveEntityPath filter management', () => {
        it('removes a filter via removeFilter', async () => {
            const transform: Transform = {
                type: 'resolve_entity_path',
                target_path: 'instance_path',
                entity_type: 'statistics_instance',
                filters: {
                    license_key_id: { kind: 'field', field: 'license_key_id' },
                },
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: transform },
            })
            await nextTick()

            const deleteButtons = wrapper.findAllComponents({ name: 'VBtn' })
            const removeBtn = deleteButtons.find(b => b.props('icon') === 'mdi-delete')
            expect(removeBtn).toBeTruthy()
            await removeBtn!.trigger('click')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'resolve_entity_path') {
                expect(Object.keys(updated.filters)).toEqual([])
            }
        })

        it('changes a filter kind from field to const_string', async () => {
            const transform: Transform = {
                type: 'resolve_entity_path',
                target_path: 'instance_path',
                entity_type: 'statistics_instance',
                filters: {
                    license_key_id: { kind: 'field', field: 'license_key_id' },
                },
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: transform },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const kindSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                return label === 'workflows.dsl.value_kind'
            })
            expect(kindSelect).toBeTruthy()
            await kindSelect!.vm.$emit('update:modelValue', 'const_string')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'resolve_entity_path') {
                expect(updated.filters.license_key_id).toEqual({ kind: 'const_string', value: '' })
            }
        })

        it('updates a field-kind filter value via select when availableFields provided', async () => {
            const transform: Transform = {
                type: 'resolve_entity_path',
                target_path: 'instance_path',
                entity_type: 'statistics_instance',
                filters: {
                    license_key_id: { kind: 'field', field: '' },
                },
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: transform, availableFields: ['a', 'b'] },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const valueSelect = selects.find(s => {
                const label = s.props('label') as string | undefined
                const items = s.props('items') as string[] | undefined
                return label === 'workflows.dsl.value' && Array.isArray(items)
            })
            expect(valueSelect).toBeTruthy()
            await valueSelect!.vm.$emit('update:modelValue', 'b')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'resolve_entity_path') {
                expect(updated.filters.license_key_id).toEqual({ kind: 'field', field: 'b' })
            }
        })

        it('updates a field-kind filter value via text field when no availableFields', async () => {
            const transform: Transform = {
                type: 'resolve_entity_path',
                target_path: 'instance_path',
                entity_type: 'statistics_instance',
                filters: {
                    license_key_id: { kind: 'field', field: '' },
                },
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: transform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const valueField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.value'
            )
            expect(valueField).toBeTruthy()
            await valueField!.vm.$emit('update:modelValue', 'manual_field')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'resolve_entity_path') {
                expect(updated.filters.license_key_id).toEqual({
                    kind: 'field',
                    field: 'manual_field',
                })
            }
        })

        it('updates a const_string-kind filter value', async () => {
            const transform: Transform = {
                type: 'resolve_entity_path',
                target_path: 'instance_path',
                entity_type: 'statistics_instance',
                filters: {
                    status: { kind: 'const_string', value: '' },
                },
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: transform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const valueField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.value'
            )
            expect(valueField).toBeTruthy()
            await valueField!.vm.$emit('update:modelValue', 'active')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'resolve_entity_path') {
                expect(updated.filters.status).toEqual({ kind: 'const_string', value: 'active' })
            }
        })

        it('updates resolve_entity_path target_uuid and fallback_path', async () => {
            const transform: Transform = {
                type: 'resolve_entity_path',
                target_path: 'instance_path',
                entity_type: 'statistics_instance',
                filters: {},
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: transform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const uuidField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.target_uuid'
            )
            const fallbackField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.fallback_path'
            )
            expect(uuidField).toBeTruthy()
            expect(fallbackField).toBeTruthy()

            await uuidField!.vm.$emit('update:modelValue', 'entity_uuid')
            await nextTick()
            let emitted = wrapper.emitted('update:modelValue')
            let updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'resolve_entity_path') {
                expect(updated.target_uuid).toBe('entity_uuid')
            }

            await fallbackField!.vm.$emit('update:modelValue', '/fallback')
            await nextTick()
            emitted = wrapper.emitted('update:modelValue')
            updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'resolve_entity_path') {
                expect(updated.fallback_path).toBe('/fallback')
            }
        })
    })

    describe('GetOrCreateEntity additional fields', () => {
        it('updates target_uuid and path_separator', async () => {
            const transform: Transform = {
                type: 'get_or_create_entity',
                target_path: 'instance_path',
                entity_type: 'statistics_instance',
                path_template: '/statistics_instance/{license_key_id}',
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: transform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const uuidField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.target_uuid'
            )
            const separatorField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.path_separator'
            )
            expect(uuidField).toBeTruthy()
            expect(separatorField).toBeTruthy()

            await uuidField!.vm.$emit('update:modelValue', 'instance_uuid')
            await nextTick()
            let emitted = wrapper.emitted('update:modelValue')
            let updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'get_or_create_entity') {
                expect(updated.target_uuid).toBe('instance_uuid')
            }

            await separatorField!.vm.$emit('update:modelValue', '-')
            await nextTick()
            emitted = wrapper.emitted('update:modelValue')
            updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'get_or_create_entity') {
                expect(updated.path_separator).toBe('-')
            }
        })
    })

    describe('BuildPath field transforms', () => {
        const buildPathTransform: Transform = {
            type: 'build_path',
            target: 'instance_path',
            template: '/statistics_instance/{license_key_id}',
            separator: '/',
            field_transforms: { license_key_id: 'slug' },
        }

        it('updates the build_path separator', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: buildPathTransform },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const separatorField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.separator'
            )
            expect(separatorField).toBeTruthy()
            await separatorField!.vm.$emit('update:modelValue', '_')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'build_path') {
                expect(updated.separator).toBe('_')
            }
        })

        it('renders existing field transforms in the mapping table', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: buildPathTransform },
            })
            await nextTick()

            const rows = wrapper.findAll('tbody tr')
            expect(rows.length).toBe(1)
        })

        it('adds a new field transform via the add_filter button', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: buildPathTransform },
            })
            await nextTick()

            const addButton = wrapper.findAll('button').find(b => b.text().includes('add_filter'))
            expect(addButton).toBeTruthy()
            await addButton!.trigger('click')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            // New empty pair has no key/value, so it's filtered out of the result
            if (updated.type === 'build_path') {
                expect(updated.field_transforms).toEqual({ license_key_id: 'slug' })
            }
        })

        it('updates a field transform pair via the mapping table', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: buildPathTransform },
            })
            await nextTick()

            const mappingTable = wrapper.findComponent({ name: 'MappingTable' })
            expect(mappingTable.exists()).toBe(true)
            await mappingTable.vm.$emit('update-pair', 0, { k: 'new_field', v: 'uppercase' })
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'build_path') {
                expect(updated.field_transforms).toEqual({ new_field: 'uppercase' })
            }
        })

        it('deletes a field transform pair via the mapping table', async () => {
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: buildPathTransform },
            })
            await nextTick()

            const mappingTable = wrapper.findComponent({ name: 'MappingTable' })
            await mappingTable.vm.$emit('delete-pair', 0)
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            const updated = emitted![emitted!.length - 1][0] as Transform
            if (updated.type === 'build_path') {
                expect(updated.field_transforms).toBeUndefined()
            }
        })
    })

    describe('Authenticate and SendEmail child editors', () => {
        it('renders AuthenticateTransformEditor for authenticate type and forwards updates', async () => {
            const transform: Transform = {
                type: 'authenticate',
                entity_type: '',
                identifier_field: '',
                password_field: '',
                input_identifier: '',
                input_password: '',
                target_token: '',
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: transform, availableFields: ['a'] },
            })
            await nextTick()

            const authEditor = wrapper.findComponent({ name: 'AuthenticateTransformEditor' })
            expect(authEditor.exists()).toBe(true)
            expect(authEditor.props('availableFields')).toEqual(['a'])

            const updatedTransform: Transform = { ...transform, entity_type: 'user' }
            await authEditor.vm.$emit('update:modelValue', updatedTransform)
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            expect(emitted![emitted!.length - 1][0]).toEqual(updatedTransform)
        })

        it('renders SendEmailTransformEditor for send_email type and forwards updates', async () => {
            mockWorkflowMailConfigured.value = true
            const transform: Transform = {
                type: 'send_email',
                template_uuid: '',
                to: [],
                target_status: '',
            }
            const wrapper = mount(DslTransformEditor, {
                props: { modelValue: transform, availableFields: ['a'] },
            })
            await nextTick()

            const sendEmailEditor = wrapper.findComponent({ name: 'SendEmailTransformEditor' })
            expect(sendEmailEditor.exists()).toBe(true)
            expect(sendEmailEditor.props('availableFields')).toEqual(['a'])

            const updatedTransform: Transform = { ...transform, target_status: 'sent' }
            await sendEmailEditor.vm.$emit('update:modelValue', updatedTransform)
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            expect(emitted![emitted!.length - 1][0]).toEqual(updatedTransform)
        })
    })
})
