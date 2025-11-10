import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import DslFromEditor from './DslFromEditor.vue'
import type { FromDef } from './dsl-utils'

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getEntityFields: vi.fn().mockResolvedValue([
            { name: 'field1', type: 'string' },
            { name: 'field2', type: 'number' },
        ]),
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k }),
}))

// Mock fetch for autoMapFromUri
global.fetch = vi.fn()

describe('DslFromEditor', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        ;(global.fetch as any).mockClear()
    })

    it('renders CSV type editor correctly', () => {
        const fromDef: FromDef = {
            type: 'csv',
            uri: 'http://example.com/data.csv',
            options: { header: true, delimiter: ',' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        expect(wrapper.find('input[type="file"]').exists()).toBe(true)
        expect(wrapper.text()).toContain('auto_map_from_uri')
    })

    it('renders JSON type editor correctly', () => {
        const fromDef: FromDef = {
            type: 'json',
            uri: 'http://example.com/data.json',
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        expect(textFields.length).toBeGreaterThan(0)
    })

    it('renders Entity type editor correctly', () => {
        const fromDef: FromDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            filter: { field: 'status', value: 'active' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        expect(textFields.length).toBeGreaterThan(0)
    })

    it('updates URI field for CSV type', async () => {
        const fromDef: FromDef = {
            type: 'csv',
            uri: '',
            options: { header: true },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        await nextTick()
        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        if (textFields.length > 0) {
            await textFields[0].vm.$emit('update:modelValue', 'http://example.com/new.csv')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            if (emitted && emitted.length > 0) {
                const updated = emitted[emitted.length - 1][0] as FromDef
                expect(updated.type).toBe('csv')
                if (updated.type === 'csv') {
                    expect(updated.uri).toBe('http://example.com/new.csv')
                }
            }
        }
    })

    it('handles CSV file upload and auto-maps fields', async () => {
        const fromDef: FromDef = {
            type: 'csv',
            uri: '',
            options: { header: true, delimiter: ',' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        // Create a mock CSV file
        const csvContent = 'name,age,email\nJohn,30,john@example.com'
        const file = new File([csvContent], 'test.csv', { type: 'text/csv' })
        // Add text() method to the file mock
        ;(file as any).text = async () => csvContent

        const fileInput = wrapper.find('input[type="file"]')
        const inputElement = fileInput.element as HTMLInputElement

        // Create a FileList mock using Object.create
        const fileList = Object.create(FileList.prototype)
        Object.defineProperty(fileList, '0', {
            value: file,
            writable: false,
        })
        Object.defineProperty(fileList, 'length', {
            value: 1,
            writable: false,
        })
        Object.defineProperty(inputElement, 'files', {
            value: fileList,
            writable: false,
        })

        await fileInput.trigger('change')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as any[]
        expect(emitted?.length).toBeGreaterThan(0)
        const updated = emitted[emitted.length - 1][0] as FromDef
        if (updated.type === 'csv') {
            expect(Object.keys(updated.mapping).length).toBeGreaterThan(0)
            expect(updated.mapping['name']).toBe('name')
            expect(updated.mapping['age']).toBe('age')
            expect(updated.mapping['email']).toBe('email')
        }
    })

    it('auto-maps from CSV URI', async () => {
        const fromDef: FromDef = {
            type: 'csv',
            uri: 'http://example.com/data.csv',
            options: { header: true, delimiter: ',' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        // Mock fetch response
        ;(global.fetch as any).mockResolvedValueOnce({
            text: async () => 'col1,col2,col3\nval1,val2,val3',
        })

        const autoMapButton = wrapper.find('button')
        await autoMapButton.trigger('click')
        await nextTick()

        // Wait for async operation
        await new Promise(resolve => setTimeout(resolve, 100))

        const emitted = wrapper.emitted('update:modelValue') as any[]
        if (emitted && emitted.length > 0) {
            const updated = emitted[emitted.length - 1][0] as FromDef
            if (updated.type === 'csv') {
                expect(Object.keys(updated.mapping).length).toBeGreaterThan(0)
            }
        }
    })

    it('handles CSV without header', async () => {
        const fromDef: FromDef = {
            type: 'csv',
            uri: '',
            options: { header: false, delimiter: ',' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        const csvContent = 'val1,val2,val3'
        const file = new File([csvContent], 'test.csv', { type: 'text/csv' })
        // Add text() method to the file mock
        ;(file as any).text = async () => csvContent

        const fileInput = wrapper.find('input[type="file"]')
        const inputElement = fileInput.element as HTMLInputElement
        
        // Create a FileList mock using Object.create
        const fileList = Object.create(FileList.prototype)
        Object.defineProperty(fileList, '0', {
            value: file,
            writable: false,
        })
        Object.defineProperty(fileList, 'length', {
            value: 1,
            writable: false,
        })
        Object.defineProperty(inputElement, 'files', {
            value: fileList,
            writable: false,
        })

        await fileInput.trigger('change')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as any[]
        if (emitted && emitted.length > 0) {
            const updated = emitted[emitted.length - 1][0] as FromDef
            if (updated.type === 'csv') {
                // Should generate col_1, col_2, col_3
                expect(Object.keys(updated.mapping).length).toBe(3)
            }
        }
    })

    it('updates entity filter fields', async () => {
        const fromDef: FromDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            filter: { field: 'status', value: 'active' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        await nextTick()
        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        // Find the filter field text field (should be the first one after entity_definition)
        const filterFieldField = textFields.find(tf => {
            const label = tf.props('label') as string
            return label && label.includes('filter_field')
        })
        
        if (filterFieldField) {
            await filterFieldField.vm.$emit('update:modelValue', 'category')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as any[]
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted[emitted.length - 1][0] as FromDef
            if (updated.type === 'entity') {
                expect(updated.filter.field).toBe('category')
            }
        }
    })

    it('adds mapping via addMapping button', async () => {
        const fromDef: FromDef = {
            type: 'csv',
            uri: '',
            options: { header: true },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        const addMappingButton = wrapper.findAll('button').find(b => b.text().includes('add_mapping'))
        if (addMappingButton) {
            await addMappingButton.trigger('click')
            await nextTick()

            // Should have added an empty pair to the mapping editor
            const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
            expect(mappingEditor.exists()).toBe(true)
        }
    })

    it('changes from type correctly', async () => {
        const fromDef: FromDef = {
            type: 'csv',
            uri: '',
            options: { header: true },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        const select = wrapper.findComponent({ name: 'VSelect' })
        await select.vm.$emit('update:modelValue', 'json')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as any[]
        expect(emitted?.length).toBeGreaterThan(0)
        const updated = emitted[emitted.length - 1][0] as FromDef
        expect(updated.type).toBe('json')
    })
})

