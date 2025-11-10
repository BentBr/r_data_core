import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import DslToEditor from './DslToEditor.vue'
import type { ToDef } from './dsl-utils'

const mockGetEntityFields = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getEntityFields: (...args: any[]) => mockGetEntityFields(...args),
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k }),
}))

vi.mock('@/composables/useEntityDefinitions', () => ({
    useEntityDefinitions: () => ({
        entityDefinitions: {
            value: [
                { entity_type: 'test_entity', display_name: 'Test Entity' },
                { entity_type: 'another_entity', display_name: 'Another Entity' },
            ],
        },
        loadEntityDefinitions: vi.fn().mockResolvedValue(undefined),
    }),
}))

describe('DslToEditor', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockGetEntityFields.mockResolvedValue([
            { name: 'field1', type: 'string' },
            { name: 'field2', type: 'number' },
            { name: 'field3', type: 'boolean' },
        ])
    })

    it('renders CSV type editor correctly', () => {
        const toDef: ToDef = {
            type: 'csv',
            output: 'api',
            options: { header: true, delimiter: ',' },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        expect(selects.length).toBeGreaterThan(0)
    })

    it('renders JSON type editor correctly', () => {
        const toDef: ToDef = {
            type: 'json',
            output: 'api',
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        expect(selects.length).toBeGreaterThan(0)
    })

    it('renders Entity type editor correctly', async () => {
        const toDef: ToDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            path: '/test',
            mode: 'create',
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100)) // Wait for async field loading

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        expect(selects.length).toBeGreaterThan(0)
    })

    it('loads entity fields when entity definition is selected', async () => {
        const toDef: ToDef = {
            type: 'entity',
            entity_definition: '',
            path: '',
            mode: 'create',
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Clear previous calls
        mockGetEntityFields.mockClear()
        
        // Update the modelValue to trigger the watch
        const updatedToDef: ToDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            path: '',
            mode: 'create',
            mapping: {},
        }
        await wrapper.setProps({ modelValue: updatedToDef })
        await nextTick()
        // Wait for the watch to trigger and the async loadEntityFields to complete
        await new Promise(resolve => setTimeout(resolve, 300))

        // The function should be called from the watch
        expect(mockGetEntityFields).toHaveBeenCalledWith('test_entity')
    })

    it('displays entity fields in mapping editor', async () => {
        const toDef: ToDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            path: '/test',
            mode: 'create',
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
        expect(mappingEditor.exists()).toBe(true)
        expect(mappingEditor.props('useSelectForLeft')).toBe(true)
        expect(mappingEditor.props('leftItems')).toEqual(['field1', 'field2', 'field3'])
    })

    it('filters out system fields from entity target fields', async () => {
        mockGetEntityFields.mockResolvedValueOnce([
            { name: 'uuid', type: 'string' },
            { name: 'field1', type: 'string' },
            { name: 'created_at', type: 'timestamp' },
            { name: 'updated_at', type: 'timestamp' },
            { name: 'field2', type: 'number' },
        ])

        const toDef: ToDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            path: '/test',
            mode: 'create',
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
        const leftItems = mappingEditor.props('leftItems') as string[]
        expect(leftItems).not.toContain('uuid')
        expect(leftItems).not.toContain('created_at')
        expect(leftItems).not.toContain('updated_at')
        expect(leftItems).toContain('field1')
        expect(leftItems).toContain('field2')
    })

    it('updates output field for CSV type', async () => {
        const toDef: ToDef = {
            type: 'csv',
            output: 'api',
            options: { header: true },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const outputSelect = selects.find(s => {
            const items = s.props('items') as any[]
            return items && items.some((item: any) => item.value === 'download')
        })

        if (outputSelect) {
            await outputSelect.vm.$emit('update:modelValue', 'download')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as any[]
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted[emitted.length - 1][0] as ToDef
            if (updated.type === 'csv') {
                expect(updated.output).toBe('download')
            }
        }
    })

    it('does not include output field for entity type', async () => {
        const toDef: ToDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            path: '/test',
            mode: 'create',
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        await nextTick()

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const hasOutputSelect = selects.some(s => {
            const items = s.props('items') as any[]
            return items && items.some((item: any) => item.value === 'api' || item.value === 'download')
        })

        expect(hasOutputSelect).toBe(false)
    })

    it('updates entity mode', async () => {
        const toDef: ToDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            path: '/test',
            mode: 'create',
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        await nextTick()

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const modeSelect = selects.find(s => {
            const items = s.props('items') as any[]
            return items && items.some((item: any) => item.value === 'update')
        })

        if (modeSelect) {
            await modeSelect.vm.$emit('update:modelValue', 'update')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as any[]
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted[emitted.length - 1][0] as ToDef
            if (updated.type === 'entity') {
                expect(updated.mode).toBe('update')
            }
        }
    })

    it('shows update_key field when mode is update', async () => {
        const toDef: ToDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            path: '/test',
            mode: 'update',
            update_key: 'entity_key',
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        await nextTick()

        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        const hasUpdateKeyField = textFields.some(tf => {
            const label = tf.props('label') as string
            return label && label.includes('update_key')
        })

        expect(hasUpdateKeyField).toBe(true)
    })

    it('adds mapping via addMapping button', async () => {
        const toDef: ToDef = {
            type: 'csv',
            output: 'api',
            options: { header: true },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        const addMappingButton = wrapper.findAll('button').find(b => b.text().includes('add_mapping'))
        if (addMappingButton) {
            await addMappingButton.trigger('click')
            await nextTick()

            const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
            expect(mappingEditor.exists()).toBe(true)
        }
    })

    it('changes to type correctly', async () => {
        const toDef: ToDef = {
            type: 'csv',
            output: 'api',
            options: { header: true },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const typeSelect = selects[0]
        await typeSelect.vm.$emit('update:modelValue', 'json')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as any[]
        expect(emitted?.length).toBeGreaterThan(0)
        const updated = emitted[emitted.length - 1][0] as ToDef
        expect(updated.type).toBe('json')
        if (updated.type === 'json') {
            expect(updated.output).toBe('api')
        }
    })
})

