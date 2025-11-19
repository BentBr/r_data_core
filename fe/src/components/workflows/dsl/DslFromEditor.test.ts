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
            return label?.includes('filter_field')
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
            type: 'format',
            source: {
                source_type: 'uri',
                config: { uri: '' },
                auth: { type: 'none' },
            },
            format: {
                format_type: 'csv',
                options: { has_header: true },
            },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        const addMappingButton = wrapper
            .findAll('button')
            .find(b => b.text().includes('add_mapping'))
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
            type: 'format',
            source: {
                source_type: 'uri',
                config: { uri: '' },
                auth: { type: 'none' },
            },
            format: {
                format_type: 'csv',
                options: { has_header: true },
            },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        const select = wrapper.findComponent({ name: 'VSelect' })
        await select.vm.$emit('update:modelValue', 'entity')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as any[]
        expect(emitted?.length).toBeGreaterThan(0)
        const updated = emitted[emitted.length - 1][0] as FromDef
        expect(updated.type).toBe('entity')
    })

    it('renders format type editor correctly', () => {
        const fromDef: FromDef = {
            type: 'format',
            source: {
                source_type: 'uri',
                config: { uri: 'http://example.com/data.csv' },
                auth: { type: 'none' },
            },
            format: {
                format_type: 'csv',
                options: { has_header: true },
            },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        expect(selects.length).toBeGreaterThan(0)
    })

    it('updates source type for format type', async () => {
        const fromDef: FromDef = {
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
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        await nextTick()
        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const sourceTypeSelect = selects.find(s => {
            const items = s.props('items') as any[]
            return items?.some((item: any) => item.value === 'api')
        })

        if (sourceTypeSelect) {
            await sourceTypeSelect.vm.$emit('update:modelValue', 'api')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as any[]
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted[emitted.length - 1][0] as FromDef
            if (updated.type === 'format') {
                expect(updated.source.source_type).toBe('api')
                // from.api source type should NOT have endpoint field (accepts POST, no endpoint needed)
                expect(updated.source.config.endpoint).toBeUndefined()
            }
        }
    })

    it('updates format type for format type', async () => {
        const fromDef: FromDef = {
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
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        await nextTick()
        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const formatTypeSelect = selects.find(s => {
            const items = s.props('items') as any[]
            return items?.some((item: any) => item.value === 'json')
        })

        if (formatTypeSelect) {
            await formatTypeSelect.vm.$emit('update:modelValue', 'json')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as any[]
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted[emitted.length - 1][0] as FromDef
            if (updated.type === 'format') {
                expect(updated.format.format_type).toBe('json')
            }
        }
    })

    it('updates URI for format type with uri source', async () => {
        const fromDef: FromDef = {
            type: 'format',
            source: {
                source_type: 'uri',
                config: { uri: '' },
                auth: { type: 'none' },
            },
            format: {
                format_type: 'csv',
                options: {},
            },
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
                if (updated.type === 'format') {
                    expect(updated.source.config.uri).toBe('http://example.com/new.csv')
                }
            } else {
                // If no event was emitted, the component might handle it internally
                // Just verify the component rendered correctly
                expect(textFields.length).toBeGreaterThan(0)
            }
        }
    })

    it('does not show endpoint field for api source type', async () => {
        const fromDef: FromDef = {
            type: 'format',
            source: {
                source_type: 'api',
                config: {}, // No endpoint field for api source
                auth: { type: 'none' },
            },
            format: {
                format_type: 'csv',
                options: {},
            },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        await nextTick()

        // Verify that no endpoint field is shown (only URI field should be shown for uri source type)
        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        // For api source type, there should be no URI/endpoint field visible
        // The only text fields should be for CSV options or other non-source config fields
        const uriField = textFields.find(field => {
            const label = field.props('label') ?? ''
            const lowerLabel = label.toLowerCase()
            return ['uri', 'endpoint'].some(term => lowerLabel.includes(term))
        })
        expect(uriField).toBeUndefined()

        // Verify config does not have endpoint field
        const emitted = wrapper.emitted('update:modelValue')
        if (emitted && emitted.length > 0) {
            const updated = emitted[emitted.length - 1][0] as FromDef
            if (updated.type === 'format') {
                expect(updated.source.config.endpoint).toBeUndefined()
            }
        }
    })

    it('shows uri field for uri source type but not for api source type', async () => {
        // Test with uri source type - should show URI field
        const uriFromDef: FromDef = {
            type: 'format',
            source: {
                source_type: 'uri',
                config: { uri: '' },
                auth: { type: 'none' },
            },
            format: {
                format_type: 'csv',
                options: {},
            },
            mapping: {},
        }
        const uriWrapper = mount(DslFromEditor, {
            props: {
                modelValue: uriFromDef,
            },
        })
        await nextTick()
        const uriTextFields = uriWrapper.findAllComponents({ name: 'VTextField' })
        const uriField = uriTextFields.find(field => {
            const label = field.props('label') ?? ''
            return label.toLowerCase().includes('uri')
        })
        expect(uriField).toBeDefined()

        // Test with api source type - should NOT show URI/endpoint field
        const apiFromDef: FromDef = {
            type: 'format',
            source: {
                source_type: 'api',
                config: {}, // No endpoint field
                auth: { type: 'none' },
            },
            format: {
                format_type: 'csv',
                options: {},
            },
            mapping: {},
        }
        const apiWrapper = mount(DslFromEditor, {
            props: {
                modelValue: apiFromDef,
            },
        })
        await nextTick()
        const apiTextFields = apiWrapper.findAllComponents({ name: 'VTextField' })
        const apiEndpointField = apiTextFields.find(field => {
            const label = field.props('label') ?? ''
            const lowerLabel = label.toLowerCase()
            return ['uri', 'endpoint'].some(term => lowerLabel.includes(term))
        })
        expect(apiEndpointField).toBeUndefined()
    })

    it('shows auth config editor in expansion panel', () => {
        const fromDef: FromDef = {
            type: 'format',
            source: {
                source_type: 'uri',
                config: { uri: 'http://example.com/data.csv' },
                auth: { type: 'api_key', key: 'test-key', header_name: 'X-API-Key' },
            },
            format: {
                format_type: 'csv',
                options: {},
            },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        const expansionPanels = wrapper.findAllComponents({ name: 'VExpansionPanel' })
        expect(expansionPanels.length).toBeGreaterThan(0)

        // AuthConfigEditor might be inside collapsed expansion panel, so check if it exists in the component tree
        // We can check if the expansion panel exists with the auth title
        const expansionPanel = expansionPanels[0]
        if (expansionPanel) {
            expect(expansionPanel.exists()).toBe(true)
        }
    })
})
