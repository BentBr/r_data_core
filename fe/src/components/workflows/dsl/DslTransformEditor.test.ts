import { describe, it, expect, beforeEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import DslTransformEditor from './DslTransformEditor.vue'
import type { Transform } from './dsl-utils'

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k }),
}))

describe('DslTransformEditor', () => {
    beforeEach(() => {
        vi.clearAllMocks()
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
            if (emitted && emitted[0]) {
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
            if (emitted && emitted[0]) {
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
            const entityTypeInput = inputs.find(
                input => input.attributes('label')?.includes('Entity Type') || false
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
            if (emitted && emitted[0]) {
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
    })
})
