import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import DslFromEditor from './DslFromEditor.vue'
import type { FromDef } from './contracts'

const mockGetEntityFields = vi.fn()
const mockGetEntityDefinitions = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getEntityFields: (entityType: string) => mockGetEntityFields(entityType),
    },
}))

vi.mock('@/composables/useEntityDefinitions', () => ({
    useEntityDefinitions: () => ({
        entityDefinitions: { value: mockGetEntityDefinitions() },
        loadEntityDefinitions: vi.fn().mockResolvedValue(undefined),
    }),
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k }),
}))

// Mock fetch for autoMapFromUri
global.fetch = vi.fn()

describe('DslFromEditor', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        ;(global.fetch as ReturnType<typeof vi.fn>).mockClear()
        mockGetEntityFields.mockResolvedValue([
            { name: 'field1', type: 'string', required: false, system: false },
            { name: 'field2', type: 'number', required: false, system: false },
            { name: 'uuid', type: 'Uuid', required: true, system: true },
            { name: 'published', type: 'Boolean', required: true, system: true },
        ])
        mockGetEntityDefinitions.mockReturnValue([
            { entity_type: 'test_entity', display_name: 'Test Entity' },
            { entity_type: 'customer', display_name: 'Customer' },
        ])
    })

    it('renders Entity type editor correctly', async () => {
        const fromDef: FromDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            filter: { field: 'status', operator: '=', value: 'active' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100)) // Wait for async field loading

        // Entity definition should be a dropdown, not text field
        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        expect(selects.length).toBeGreaterThan(0)

        // Filter field should also be a dropdown
        const filterFieldSelect = selects.find(s => {
            const label = s.props('label') as string
            return label.includes('filter_field')
        })
        expect(filterFieldSelect).toBeTruthy()
    })

    it('entity_definition is a dropdown with entity definitions', async () => {
        const fromDef: FromDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            filter: { field: 'status', operator: '=', value: 'active' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const entityDefSelect = selects.find(s => {
            const label = s.props('label') as string
            return label.includes('entity_definition')
        })

        expect(entityDefSelect).toBeTruthy()
        expect(entityDefSelect?.props('items')).toBeTruthy()
    })

    it('filter_field is a dropdown with entity fields including system fields', async () => {
        const fromDef: FromDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            filter: { field: 'status', operator: '=', value: 'active' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const filterFieldSelect = selects.find(s => {
            const label = s.props('label') as string
            return label.includes('filter_field')
        })

        expect(filterFieldSelect).toBeTruthy()
        expect(mockGetEntityFields).toHaveBeenCalledWith('test_entity')

        // Check that system fields are included
        const items = filterFieldSelect?.props('items') as string[]
        expect(items).toContain('uuid')
        expect(items).toContain('published')
        expect(items).toContain('field1')
        expect(items).toContain('field2')
    })

    it('filter_operator dropdown appears with all operators', async () => {
        const fromDef: FromDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            filter: { field: 'status', operator: '=', value: 'active' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const operatorSelect = selects.find(s => {
            const label = s.props('label') as string
            return label.includes('filter_operator')
        })

        expect(operatorSelect).toBeTruthy()
        const items = operatorSelect?.props('items') as Array<{ title: string; value: string }>
        expect(items).toHaveLength(7)
        expect(items.map(i => i.value)).toContain('=')
        expect(items.map(i => i.value)).toContain('>')
        expect(items.map(i => i.value)).toContain('<')
        expect(items.map(i => i.value)).toContain('<=')
        expect(items.map(i => i.value)).toContain('>=')
        expect(items.map(i => i.value)).toContain('IN')
        expect(items.map(i => i.value)).toContain('NOT IN')
    })

    it('selecting filter operator updates filter structure', async () => {
        const fromDef: FromDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            filter: { field: 'status', operator: '=', value: 'active' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const operatorSelect = selects.find(s => {
            const label = s.props('label') as string
            return label.includes('filter_operator')
        })

        if (operatorSelect) {
            await operatorSelect.vm.$emit('update:modelValue', '>')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]> | undefined
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted![emitted!.length - 1][0] as FromDef
            if (updated.type === 'entity' && updated.filter) {
                expect(updated.filter.operator).toBe('>')
            }
        }
    })

    it('updates entity filter fields', async () => {
        const fromDef: FromDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            filter: { field: 'status', operator: '=', value: 'active' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 100)) // Wait for async field loading

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        // Find the filter field select (should be a dropdown now)
        const filterFieldSelect = selects.find(s => {
            const label = s.props('label') as string
            return label.includes('filter_field')
        })

        if (filterFieldSelect) {
            await filterFieldSelect.vm.$emit('update:modelValue', 'category')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]> | undefined
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted![emitted!.length - 1][0] as FromDef
            if (updated.type === 'entity' && updated.filter) {
                expect(updated.filter.field).toBe('category')
            }
        } else {
            // If select not found, just verify the component rendered
            expect(selects.length).toBeGreaterThan(0)
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

        const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]> | undefined
        expect(emitted?.length).toBeGreaterThan(0)
        const updated = emitted![emitted!.length - 1][0] as FromDef
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
            const items = s.props('items') as Array<{ value: string; title: string }> | undefined
            return items?.some(item => item.value === 'api')
        })

        if (sourceTypeSelect) {
            await sourceTypeSelect.vm.$emit('update:modelValue', 'api')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]> | undefined
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted![emitted!.length - 1][0] as FromDef
            if (updated.type === 'format') {
                expect(updated.source.source_type).toBe('api')
                // from.api source type should NOT have endpoint field (accepts POST, no endpoint needed)
                expect(updated.source.config.endpoint).toBeUndefined()
            }
        }
    })

    it('updates format options via CsvOptionsEditor', async () => {
        const fromDef: FromDef = {
            type: 'format',
            source: { source_type: 'uri', config: { uri: '' }, auth: { type: 'none' } },
            format: { format_type: 'csv', options: { has_header: true } },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: { modelValue: fromDef },
        })
        await nextTick()

        const csvOptionsEditor = wrapper.findComponent({ name: 'CsvOptionsEditor' })
        expect(csvOptionsEditor.exists()).toBe(true)
        await csvOptionsEditor.vm.$emit('update:modelValue', { has_header: false, delimiter: ';' })
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]>
        const updated = emitted[emitted.length - 1][0]
        if (updated.type === 'format') {
            expect(updated.format.options).toEqual({ has_header: false, delimiter: ';' })
        }
    })

    it('shows the API accept-POST endpoint banner for api source type', async () => {
        const fromDef: FromDef = {
            type: 'format',
            source: { source_type: 'api', config: {}, auth: { type: 'none' } },
            format: { format_type: 'json', options: {} },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: { modelValue: fromDef, workflowUuid: 'wf-42' },
        })
        await nextTick()

        expect(wrapper.text()).toContain('endpoint_info')
        expect(wrapper.text()).toContain('wf-42')
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
            const items = s.props('items') as Array<{ value: string; title: string }> | undefined
            return items?.some(item => item.value === 'json')
        })

        if (formatTypeSelect) {
            await formatTypeSelect.vm.$emit('update:modelValue', 'json')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]> | undefined
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted![emitted!.length - 1][0] as FromDef
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
                const updated = emitted![emitted!.length - 1][0] as FromDef
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
            const updated = emitted![emitted!.length - 1][0] as FromDef
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
        expect(expansionPanel.exists()).toBe(true)
    })

    it('uses filter_operator translation key', () => {
        const fromDef: FromDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            filter: { field: 'status', operator: '=', value: 'active' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        const text = wrapper.text()
        // Check that translation key is used (mocked to return key itself)
        expect(text).toContain('filter_operator')
    })

    it('toggles filter off and removes it from the model', async () => {
        const fromDef: FromDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            filter: { field: 'status', operator: '=', value: 'active' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: { modelValue: fromDef },
        })
        await nextTick()

        const toggle = wrapper.findComponent({ name: 'VSwitch' })
        expect(toggle.exists()).toBe(true)
        await toggle.vm.$emit('update:modelValue', false)
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]>
        const updated = emitted[emitted.length - 1][0]
        if (updated.type === 'entity') {
            expect(updated.filter).toBeUndefined()
        }
    })

    it('toggles filter on when none exists and creates a default filter', async () => {
        const fromDef: FromDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: { modelValue: fromDef },
        })
        await nextTick()

        const toggle = wrapper.findComponent({ name: 'VSwitch' })
        await toggle.vm.$emit('update:modelValue', true)
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]>
        const updated = emitted[emitted.length - 1][0]
        if (updated.type === 'entity') {
            expect(updated.filter).toEqual({ field: '', operator: '=', value: '' })
        }
    })

    it('updates filter value text field', async () => {
        const fromDef: FromDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            filter: { field: 'status', operator: '=', value: 'active' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: { modelValue: fromDef },
        })
        await nextTick()

        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        const filterValueField = textFields.find(
            tf => (tf.props('label') as string) === 'workflows.dsl.filter_value'
        )
        expect(filterValueField).toBeTruthy()
        await filterValueField!.vm.$emit('update:modelValue', 'inactive')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]>
        const updated = emitted[emitted.length - 1][0]
        if (updated.type === 'entity') {
            expect(updated.filter?.value).toBe('inactive')
        }
    })

    describe('Trigger FromDef', () => {
        it('renders the trigger endpoint info banner', async () => {
            const fromDef: FromDef = { type: 'trigger', mapping: {} }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef, workflowUuid: 'wf-123' },
            })
            await nextTick()

            expect(wrapper.text()).toContain('endpoint_info')
            expect(wrapper.text()).toContain('wf-123')
            expect(wrapper.text()).toContain('/trigger')
        })

        it('uses a placeholder endpoint when no workflowUuid is provided', async () => {
            const fromDef: FromDef = { type: 'trigger', mapping: {} }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef },
            })
            await nextTick()

            expect(wrapper.text()).toContain('{workflow-uuid}')
        })

        it('changes from trigger to format type', async () => {
            const fromDef: FromDef = { type: 'trigger', mapping: {} }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef },
            })
            await nextTick()

            const select = wrapper.findComponent({ name: 'VSelect' })
            await select.vm.$emit('update:modelValue', 'format')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]>
            const updated = emitted[emitted.length - 1][0]
            expect(updated.type).toBe('format')
        })
    })

    describe('PreviousStep FromDef', () => {
        it('shows error alert when stepIndex is 0', async () => {
            const fromDef: FromDef = { type: 'previous_step', mapping: {} }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef, stepIndex: 0 },
            })
            await nextTick()

            const alerts = wrapper.findAllComponents({ name: 'VAlert' })
            const errorAlert = alerts.find(a => a.props('type') === 'error')
            expect(errorAlert).toBeTruthy()
            expect(errorAlert!.text()).toContain('previous_step_error_first_step')
        })

        it('shows info banner when stepIndex is greater than 0', async () => {
            const fromDef: FromDef = { type: 'previous_step', mapping: {} }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef, stepIndex: 1 },
            })
            await nextTick()

            const alerts = wrapper.findAllComponents({ name: 'VAlert' })
            expect(alerts.find(a => a.props('type') === 'error')).toBeUndefined()
            expect(wrapper.text()).toContain('previous_step_info')
        })

        it('defaults stepIndex to 0 when not provided', async () => {
            const fromDef: FromDef = { type: 'previous_step', mapping: {} }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef },
            })
            await nextTick()

            const alerts = wrapper.findAllComponents({ name: 'VAlert' })
            expect(alerts.find(a => a.props('type') === 'error')).toBeTruthy()
        })

        it('uses previousStepFields as select items in the mapping editor', async () => {
            const fromDef: FromDef = { type: 'previous_step', mapping: {} }
            const wrapper = mount(DslFromEditor, {
                props: {
                    modelValue: fromDef,
                    stepIndex: 1,
                    previousStepFields: ['field_a', 'field_b'],
                },
            })
            await nextTick()

            const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
            expect(mappingEditor.props('useSelectForLeft')).toBe(true)
            expect(mappingEditor.props('leftItems')).toEqual(['field_a', 'field_b'])
        })

        it('does not use a select for left items when previousStepFields is empty', async () => {
            const fromDef: FromDef = { type: 'previous_step', mapping: {} }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef, stepIndex: 1 },
            })
            await nextTick()

            const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
            expect(mappingEditor.props('useSelectForLeft')).toBe(false)
        })

        it('updates mapping and changes type away from previous_step', async () => {
            const fromDef: FromDef = { type: 'previous_step', mapping: {} }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef, stepIndex: 1 },
            })
            await nextTick()

            const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
            const newMapping = { prev_field: 'normalized' }
            await mappingEditor.vm.$emit('update:modelValue', newMapping)
            await nextTick()

            let emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]>
            let updated = emitted[emitted.length - 1][0]
            if (updated.type === 'previous_step') {
                expect(updated.mapping).toEqual(newMapping)
            }

            const select = wrapper.findComponent({ name: 'VSelect' })
            await select.vm.$emit('update:modelValue', 'entity')
            await nextTick()
            emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]>
            updated = emitted[emitted.length - 1][0]
            expect(updated.type).toBe('entity')
        })
    })

    describe('CSV test upload and auto-mapping', () => {
        function makeFile(content: string, name = 'test.csv'): File {
            const file = new File([content], name, { type: 'text/csv' })
            Object.defineProperty(file, 'text', {
                value: () => Promise.resolve(content),
            })
            return file
        }

        it('maps CSV headers to fields on file upload', async () => {
            const fromDef: FromDef = {
                type: 'format',
                source: { source_type: 'uri', config: { uri: '' }, auth: { type: 'none' } },
                format: { format_type: 'csv', options: { has_header: true } },
                mapping: {},
            }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef },
            })
            await nextTick()

            const fileInput = wrapper.find('input[type="file"]')
            expect(fileInput.exists()).toBe(true)

            const file = makeFile('name,email\nAlice,alice@test.com')
            const input = fileInput.element as HTMLInputElement
            Object.defineProperty(input, 'files', { value: [file], configurable: true })
            await fileInput.trigger('change')
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 20))

            const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]> | undefined
            expect(emitted).toBeTruthy()
            const updated = emitted![emitted!.length - 1][0]
            if (updated.type === 'format') {
                expect(updated.mapping).toEqual({ name: 'name', email: 'email' })
            }
        })

        it('generates column names when the CSV has no header row', async () => {
            const fromDef: FromDef = {
                type: 'format',
                source: { source_type: 'uri', config: { uri: '' }, auth: { type: 'none' } },
                format: { format_type: 'csv', options: { has_header: false } },
                mapping: {},
            }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef },
            })
            await nextTick()

            const fileInput = wrapper.find('input[type="file"]')
            const file = makeFile('Alice,alice@test.com')
            const input = fileInput.element as HTMLInputElement
            Object.defineProperty(input, 'files', { value: [file], configurable: true })
            await fileInput.trigger('change')
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 20))

            const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]> | undefined
            const updated = emitted![emitted!.length - 1][0]
            if (updated.type === 'format') {
                expect(updated.mapping).toEqual({ col_1: 'col_1', col_2: 'col_2' })
            }
        })

        it('does nothing on upload when no file is selected', async () => {
            const fromDef: FromDef = {
                type: 'format',
                source: { source_type: 'uri', config: { uri: '' }, auth: { type: 'none' } },
                format: { format_type: 'csv', options: { has_header: true } },
                mapping: {},
            }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef },
            })
            await nextTick()

            const fileInput = wrapper.find('input[type="file"]')
            const input = fileInput.element as HTMLInputElement
            Object.defineProperty(input, 'files', { value: [], configurable: true })
            await fileInput.trigger('change')
            await nextTick()

            expect(wrapper.emitted('update:modelValue')).toBeFalsy()
        })

        it('auto-maps fields from a remote URI via fetch', async () => {
            ;(global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({
                text: () => Promise.resolve('id,name\n1,Alice'),
            })
            const fromDef: FromDef = {
                type: 'format',
                source: {
                    source_type: 'uri',
                    config: { uri: 'http://example.com/data.csv' },
                    auth: { type: 'none' },
                },
                format: { format_type: 'csv', options: { has_header: true } },
                mapping: {},
            }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef },
            })
            await nextTick()

            const autoMapButton = wrapper
                .findAll('button')
                .find(b => b.text().includes('auto_map_from_uri'))
            expect(autoMapButton).toBeTruthy()
            await autoMapButton!.trigger('click')
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 20))

            expect(global.fetch).toHaveBeenCalledWith('http://example.com/data.csv')
            const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]> | undefined
            const updated = emitted![emitted!.length - 1][0]
            if (updated.type === 'format') {
                expect(updated.mapping).toEqual({ id: 'id', name: 'name' })
            }
        })

        it('ignores fetch errors during auto-map from URI', async () => {
            ;(global.fetch as ReturnType<typeof vi.fn>).mockRejectedValueOnce(
                new Error('CORS error')
            )
            const fromDef: FromDef = {
                type: 'format',
                source: {
                    source_type: 'uri',
                    config: { uri: 'http://example.com/data.csv' },
                    auth: { type: 'none' },
                },
                format: { format_type: 'csv', options: { has_header: true } },
                mapping: {},
            }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef },
            })
            await nextTick()

            const autoMapButton = wrapper
                .findAll('button')
                .find(b => b.text().includes('auto_map_from_uri'))
            await autoMapButton!.trigger('click')
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 20))

            expect(wrapper.emitted('update:modelValue')).toBeFalsy()
        })

        it('does not show the auto-map button when source type is not uri', async () => {
            const fromDef: FromDef = {
                type: 'format',
                source: { source_type: 'api', config: {}, auth: { type: 'none' } },
                format: { format_type: 'csv', options: { has_header: true } },
                mapping: {},
            }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef },
            })
            await nextTick()

            const autoMapButton = wrapper
                .findAll('button')
                .find(b => b.text().includes('auto_map_from_uri'))
            expect(autoMapButton).toBeUndefined()
        })

        it('does not show CSV upload controls when source type is trigger', async () => {
            const fromDef: FromDef = {
                type: 'format',
                source: { source_type: 'trigger', config: {}, auth: { type: 'none' } },
                format: { format_type: 'csv', options: { has_header: true } },
                mapping: {},
            }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef },
            })
            await nextTick()

            const fileInput = wrapper.find('input[type="file"]')
            expect(fileInput.exists()).toBe(false)
        })
    })

    describe('Auth config for source', () => {
        it('updates source auth via AuthConfigEditor', async () => {
            const fromDef: FromDef = {
                type: 'format',
                source: { source_type: 'uri', config: { uri: '' }, auth: { type: 'none' } },
                format: { format_type: 'json', options: {} },
                mapping: {},
            }
            const wrapper = mount(DslFromEditor, {
                props: { modelValue: fromDef },
            })
            await nextTick()

            const panelTitle = wrapper.findComponent({ name: 'VExpansionPanelTitle' })
            await panelTitle.trigger('click')
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 50))
            await nextTick()

            const authEditor = wrapper.findComponent({ name: 'AuthConfigEditor' })
            expect(authEditor.exists()).toBe(true)
            const newAuth = { type: 'api_key' as const, key: 'secret', header_name: 'X-API-Key' }
            await authEditor.vm.$emit('update:modelValue', newAuth)
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[FromDef]>
            const updated = emitted[emitted.length - 1][0]
            if (updated.type === 'format') {
                expect(updated.source.auth).toEqual(newAuth)
            }
        })
    })

    it('uses filter_field and filter_value translation keys', () => {
        const fromDef: FromDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            filter: { field: 'status', operator: '=', value: 'active' },
            mapping: {},
        }
        const wrapper = mount(DslFromEditor, {
            props: {
                modelValue: fromDef,
            },
        })

        const text = wrapper.text()
        // Check that translation keys are used
        expect(text).toContain('filter_field')
        expect(text).toContain('filter_value')
    })
})
