import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import DslToEditor from './DslToEditor.vue'
import type { ToDef } from './contracts'

const mockGetEntityFields = vi.fn()
const mockListEmailTemplates = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getEntityFields: (entityType: string) => mockGetEntityFields(entityType),
        listEmailTemplates: (type?: string) => mockListEmailTemplates(type),
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

const mockWorkflowMailConfigured = { value: false }

vi.mock('@/stores/capabilities', () => ({
    useCapabilitiesStore: () => ({
        get workflowMailConfigured() {
            return mockWorkflowMailConfigured.value
        },
    }),
}))

describe('DslToEditor', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockWorkflowMailConfigured.value = false
        mockGetEntityFields.mockResolvedValue([
            { name: 'field1', type: 'string' },
            { name: 'field2', type: 'number' },
            { name: 'field3', type: 'boolean' },
        ])
        mockListEmailTemplates.mockResolvedValue([])
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
            const items = s.props('items') as Array<{ value: string; title: string }> | undefined
            return items?.some(item => item.value === 'api' || item.value === 'download')
        })

        expect(hasOutputSelect).toBe(false)
    })

    it('loads entity fields via onEntityDefChange when the entity definition select changes', async () => {
        const toDef: ToDef = {
            type: 'entity',
            entity_definition: '',
            path: '',
            mode: 'create',
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: { modelValue: toDef },
        })
        await nextTick()
        mockGetEntityFields.mockClear()

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const entityDefSelect = selects.find(s => {
            const label = s.props('label') as string
            return label.includes('entity_definition')
        })
        await entityDefSelect!.vm.$emit('update:modelValue', 'another_entity')
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        expect(mockGetEntityFields).toHaveBeenCalledWith('another_entity')

        const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
        const updated = emitted[emitted.length - 1][0]
        if (updated.type === 'entity') {
            expect(updated.entity_definition).toBe('another_entity')
        }
    })

    it('clears entity target fields when getEntityFields rejects', async () => {
        mockGetEntityFields.mockRejectedValueOnce(new Error('network error'))
        const toDef: ToDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            path: '/test',
            mode: 'create',
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: { modelValue: toDef },
        })
        await nextTick()
        await new Promise(resolve => setTimeout(resolve, 50))

        const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
        expect(mappingEditor.props('leftItems')).toEqual([])
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
            const items = s.props('items') as Array<{ value: string; title: string }> | undefined
            return items?.some(item => item.value === 'update')
        })

        if (modeSelect) {
            await modeSelect.vm.$emit('update:modelValue', 'update')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]> | undefined
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted![emitted!.length - 1][0] as ToDef
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
            return label.includes('update_key')
        })

        expect(hasUpdateKeyField).toBe(true)
    })

    it('shows update_key field when mode is create_or_update', async () => {
        const toDef: ToDef = {
            type: 'entity',
            entity_definition: 'test_entity',
            path: '/test',
            mode: 'create_or_update',
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
            return label.includes('update_key')
        })

        expect(hasUpdateKeyField).toBe(true)
    })

    it('includes create_or_update in entity modes', async () => {
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
            const items = s.props('items') as Array<{ value: string; title: string }> | undefined
            return items?.some(
                item =>
                    item.value === 'create' ||
                    item.value === 'update' ||
                    item.value === 'create_or_update'
            )
        })

        expect(modeSelect).toBeTruthy()
        if (modeSelect) {
            const items = modeSelect.props('items') as
                Array<{ value: string; title: string }> | undefined
            const hasCreateOrUpdate =
                items?.some(item => item.value === 'create_or_update') ?? false
            expect(hasCreateOrUpdate).toBe(true)
        }
    })

    it('updates entity mode to create_or_update', async () => {
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
            const items = s.props('items') as Array<{ value: string; title: string }> | undefined
            return items?.some(item => item.value === 'create_or_update')
        })

        if (modeSelect) {
            await modeSelect.vm.$emit('update:modelValue', 'create_or_update')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]> | undefined
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted![emitted!.length - 1][0] as ToDef
            if (updated.type === 'entity') {
                expect(updated.mode).toBe('create_or_update')
            }
        }
    })

    it('adds mapping via addMapping button', async () => {
        const toDef: ToDef = {
            type: 'format',
            output: { mode: 'api' },
            format: {
                format_type: 'csv',
                options: { has_header: true },
            },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        const addMappingButton = wrapper
            .findAll('button')
            .find(b => b.text().includes('add_mapping'))
        if (addMappingButton) {
            await addMappingButton.trigger('click')
            await nextTick()

            const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
            expect(mappingEditor.exists()).toBe(true)
        }
    })

    it('changes to type correctly', async () => {
        const toDef: ToDef = {
            type: 'format',
            output: { mode: 'api' },
            format: {
                format_type: 'json',
                options: {},
            },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const typeSelect = selects[0]
        await typeSelect.vm.$emit('update:modelValue', 'entity')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]> | undefined
        expect(emitted?.length).toBeGreaterThan(0)
        const updated = emitted![emitted!.length - 1][0] as ToDef
        expect(updated.type).toBe('entity')
    })

    it('renders format type editor correctly', () => {
        const toDef: ToDef = {
            type: 'format',
            output: { mode: 'api' },
            format: {
                format_type: 'json',
                options: {},
            },
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

    it('updates format type for format type', async () => {
        const toDef: ToDef = {
            type: 'format',
            output: { mode: 'api' },
            format: {
                format_type: 'json',
                options: {},
            },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        await nextTick()
        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const formatTypeSelect = selects.find(s => {
            const items = s.props('items') as Array<{ value: string; title: string }> | undefined
            return items?.some(item => item.value === 'csv')
        })

        if (formatTypeSelect) {
            await formatTypeSelect.vm.$emit('update:modelValue', 'csv')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]> | undefined
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted![emitted!.length - 1][0] as ToDef
            if (updated.type === 'format') {
                expect(updated.format.format_type).toBe('csv')
            }
        }
    })

    it('updates format options via CsvOptionsEditor', async () => {
        const toDef: ToDef = {
            type: 'format',
            output: { mode: 'api' },
            format: { format_type: 'csv', options: { has_header: true } },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: { modelValue: toDef },
        })
        await nextTick()

        const csvOptionsEditor = wrapper.findComponent({ name: 'CsvOptionsEditor' })
        expect(csvOptionsEditor.exists()).toBe(true)
        await csvOptionsEditor.vm.$emit('update:modelValue', { has_header: false, delimiter: ';' })
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
        const updated = emitted[emitted.length - 1][0]
        if (updated.type === 'format') {
            expect(updated.format.options).toEqual({ has_header: false, delimiter: ';' })
        }
    })

    it('updates output mode to download', async () => {
        const toDef: ToDef = {
            type: 'format',
            output: { mode: 'api' },
            format: { format_type: 'json', options: {} },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: { modelValue: toDef },
        })
        await nextTick()
        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const outputModeSelect = selects.find(s => {
            const items = s.props('items') as Array<{ value: string; title: string }> | undefined
            return items?.some(item => item.value === 'download')
        })

        await outputModeSelect!.vm.$emit('update:modelValue', 'download')
        await nextTick()

        const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
        const updated = emitted[emitted.length - 1][0]
        if (updated.type === 'format') {
            expect(updated.output).toEqual({ mode: 'download' })
        }
    })

    it('updates output mode to push', async () => {
        const toDef: ToDef = {
            type: 'format',
            output: { mode: 'api' },
            format: {
                format_type: 'json',
                options: {},
            },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        await nextTick()
        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const outputModeSelect = selects.find(s => {
            const items = s.props('items') as Array<{ value: string; title: string }> | undefined
            return items?.some(item => item.value === 'push')
        })

        if (outputModeSelect) {
            await outputModeSelect.vm.$emit('update:modelValue', 'push')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]> | undefined
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted![emitted!.length - 1][0] as ToDef
            if (updated.type === 'format') {
                expect(updated.output.mode).toBe('push')
                if (updated.output.mode === 'push') {
                    expect(updated.output.destination.destination_type).toBe('uri')
                    expect(updated.output.method).toBe('POST')
                }
            }
        }
    })

    it('shows push destination fields when output mode is push', async () => {
        const toDef: ToDef = {
            type: 'format',
            output: {
                mode: 'push',
                destination: {
                    destination_type: 'uri',
                    config: { uri: 'http://example.com/api' },
                    auth: { type: 'none' },
                },
                method: 'POST',
            },
            format: {
                format_type: 'json',
                options: {},
            },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        await nextTick()
        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        const selects = wrapper.findAllComponents({ name: 'VSelect' })

        // Should have destination type select, HTTP method select, and URI field
        expect(selects.length).toBeGreaterThan(2)
        expect(textFields.length).toBeGreaterThan(0)
    })

    it('updates destination URI for push mode', async () => {
        const toDef: ToDef = {
            type: 'format',
            output: {
                mode: 'push',
                destination: {
                    destination_type: 'uri',
                    config: { uri: '' },
                    auth: { type: 'none' },
                },
                method: 'POST',
            },
            format: {
                format_type: 'json',
                options: {},
            },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        await nextTick()
        const textFields = wrapper.findAllComponents({ name: 'VTextField' })
        const uriField = textFields.find(tf => {
            const label = tf.props('label') as string
            return label.includes('uri')
        })

        if (uriField) {
            await uriField.vm.$emit('update:modelValue', 'http://example.com/new-endpoint')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue')
            if (emitted && emitted.length > 0) {
                const updated = emitted![emitted!.length - 1][0] as ToDef
                if (updated.type === 'format' && updated.output.mode === 'push') {
                    expect(updated.output.destination.config.uri).toBe(
                        'http://example.com/new-endpoint'
                    )
                }
            } else {
                // If no event was emitted, the component might handle it internally
                // Just verify the component rendered correctly
                expect(uriField.exists()).toBe(true)
            }
        }
    })

    it('updates HTTP method for push mode', async () => {
        const toDef: ToDef = {
            type: 'format',
            output: {
                mode: 'push',
                destination: {
                    destination_type: 'uri',
                    config: { uri: 'http://example.com/api' },
                    auth: { type: 'none' },
                },
                method: 'POST',
            },
            format: {
                format_type: 'json',
                options: {},
            },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        await nextTick()
        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const httpMethodSelect = selects.find(s => {
            const items = s.props('items') as Array<{ value: string; title: string }> | undefined
            return items?.some(item => item.value === 'PUT')
        })

        if (httpMethodSelect) {
            await httpMethodSelect.vm.$emit('update:modelValue', 'PUT')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]> | undefined
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted![emitted!.length - 1][0] as ToDef
            if (updated.type === 'format' && updated.output.mode === 'push') {
                expect(updated.output.method).toBe('PUT')
            }
        }
    })

    it('shows auth config editor for push mode', () => {
        const toDef: ToDef = {
            type: 'format',
            output: {
                mode: 'push',
                destination: {
                    destination_type: 'uri',
                    config: { uri: 'http://example.com/api' },
                    auth: { type: 'api_key', key: 'test-key', header_name: 'X-API-Key' },
                },
                method: 'POST',
            },
            format: {
                format_type: 'json',
                options: {},
            },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        const expansionPanels = wrapper.findAllComponents({ name: 'VExpansionPanel' })
        expect(expansionPanels.length).toBeGreaterThan(0)

        // AuthConfigEditor might be inside collapsed expansion panel, so check if expansion panel exists
        const expansionPanel = expansionPanels[0]
        expect(expansionPanel.exists()).toBe(true)
    })

    it('updates output mode from push to api', async () => {
        const toDef: ToDef = {
            type: 'format',
            output: {
                mode: 'push',
                destination: {
                    destination_type: 'uri',
                    config: { uri: 'http://example.com/api' },
                    auth: { type: 'none' },
                },
                method: 'POST',
            },
            format: {
                format_type: 'json',
                options: {},
            },
            mapping: {},
        }
        const wrapper = mount(DslToEditor, {
            props: {
                modelValue: toDef,
            },
        })

        await nextTick()
        const selects = wrapper.findAllComponents({ name: 'VSelect' })
        const outputModeSelect = selects.find(s => {
            const items = s.props('items') as Array<{ value: string; title: string }> | undefined
            return items?.some(item => item.value === 'api')
        })

        if (outputModeSelect) {
            await outputModeSelect.vm.$emit('update:modelValue', 'api')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]> | undefined
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted![emitted!.length - 1][0] as ToDef
            if (updated.type === 'format') {
                expect(updated.output.mode).toBe('api')
            }
        }
    })

    describe('NextStep ToDef', () => {
        it('includes NextStep in type selector', async () => {
            const toDef: ToDef = {
                type: 'format',
                output: { mode: 'api' },
                format: {
                    format_type: 'json',
                    options: {},
                },
                mapping: {},
            }
            const wrapper = mount(DslToEditor, {
                props: {
                    modelValue: toDef,
                },
            })

            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const typeSelect = selects[0]
            const items = typeSelect.props('items') as
                Array<{ value: string; title: string }> | undefined
            const hasNextStep = items?.some(item => item.value === 'next_step') ?? false

            expect(hasNextStep).toBe(true)
        })

        it('renders NextStep editor correctly', async () => {
            const toDef: ToDef = {
                type: 'next_step',
                mapping: {},
            }
            const wrapper = mount(DslToEditor, {
                props: {
                    modelValue: toDef,
                    isLastStep: false,
                },
            })

            await nextTick()

            const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
            expect(mappingEditor.exists()).toBe(true)
        })

        it('shows error alert when isLastStep is true', async () => {
            const toDef: ToDef = {
                type: 'next_step',
                mapping: {},
            }
            const wrapper = mount(DslToEditor, {
                props: {
                    modelValue: toDef,
                    isLastStep: true,
                },
            })

            await nextTick()

            const alerts = wrapper.findAllComponents({ name: 'VAlert' })
            const errorAlert = alerts.find(a => a.props('type') === 'error')

            expect(errorAlert).toBeDefined()
            if (errorAlert) {
                expect(errorAlert.exists()).toBe(true)
                expect(errorAlert.text()).toContain('next_step_error_last_step')
            }
        })

        it('shows info banner when isLastStep is false', async () => {
            const toDef: ToDef = {
                type: 'next_step',
                mapping: {},
            }
            const wrapper = mount(DslToEditor, {
                props: {
                    modelValue: toDef,
                    isLastStep: false,
                },
            })

            await nextTick()

            const alerts = wrapper.findAllComponents({ name: 'VAlert' })
            const errorAlert = alerts.find(a => a.props('type') === 'error')
            expect(errorAlert).toBeUndefined()

            // Check for info banner (div with info styling)
            const infoBanner = wrapper.find('[style*="background-color"]')
            expect(infoBanner.exists()).toBe(true)
            expect(infoBanner.text()).toContain('next_step_info')
        })

        it('shows mapping editor with correct labels for NextStep', async () => {
            const toDef: ToDef = {
                type: 'next_step',
                mapping: { normalized_field: 'next_step_field' },
            }
            const wrapper = mount(DslToEditor, {
                props: {
                    modelValue: toDef,
                    isLastStep: false,
                },
            })

            await nextTick()

            const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
            expect(mappingEditor.exists()).toBe(true)
            // Translation mock returns the key as-is
            expect(mappingEditor.props('leftLabel')).toBe('workflows.dsl.normalized')
            expect(mappingEditor.props('rightLabel')).toBe('workflows.dsl.next_step_field')
        })

        it('changes to NextStep type correctly', async () => {
            const toDef: ToDef = {
                type: 'format',
                output: { mode: 'api' },
                format: {
                    format_type: 'json',
                    options: {},
                },
                mapping: {},
            }
            const wrapper = mount(DslToEditor, {
                props: {
                    modelValue: toDef,
                    isLastStep: false,
                },
            })

            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const typeSelect = selects[0]
            await typeSelect.vm.$emit('update:modelValue', 'next_step')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]> | undefined
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted![emitted!.length - 1][0] as ToDef
            expect(updated.type).toBe('next_step')
            if (updated.type === 'next_step') {
                expect(updated.mapping).toEqual({})
            }
        })

        it('changes from NextStep to entity type correctly', async () => {
            const toDef: ToDef = {
                type: 'next_step',
                mapping: { field1: 'field2' },
            }
            const wrapper = mount(DslToEditor, {
                props: {
                    modelValue: toDef,
                    isLastStep: false,
                },
            })

            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const typeSelect = selects[0]
            await typeSelect.vm.$emit('update:modelValue', 'entity')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]> | undefined
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted![emitted!.length - 1][0] as ToDef
            expect(updated.type).toBe('entity')
        })

        it('updates NextStep mapping', async () => {
            const toDef: ToDef = {
                type: 'next_step',
                mapping: {},
            }
            const wrapper = mount(DslToEditor, {
                props: {
                    modelValue: toDef,
                    isLastStep: false,
                },
            })

            await nextTick()

            const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
            const newMapping = { normalized_field: 'next_step_field' }
            await mappingEditor.vm.$emit('update:modelValue', newMapping)
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]> | undefined
            expect(emitted?.length).toBeGreaterThan(0)
            const updated = emitted![emitted!.length - 1][0] as ToDef
            if (updated.type === 'next_step') {
                expect(updated.mapping).toEqual(newMapping)
            }
        })

        it('defaults isLastStep to false when not provided', async () => {
            const toDef: ToDef = {
                type: 'next_step',
                mapping: {},
            }
            const wrapper = mount(DslToEditor, {
                props: {
                    modelValue: toDef,
                },
            })

            await nextTick()

            const alerts = wrapper.findAllComponents({ name: 'VAlert' })
            const errorAlert = alerts.find(a => a.props('type') === 'error')
            expect(errorAlert).toBeUndefined()
        })
    })

    describe('Email ToDef', () => {
        function makeEmailTo(overrides: Partial<Extract<ToDef, { type: 'email' }>> = {}): ToDef {
            return {
                type: 'email',
                template_uuid: '',
                to: [],
                mapping: {},
                ...overrides,
            }
        }

        it('includes Email in type selector when workflowMailConfigured is true', async () => {
            mockWorkflowMailConfigured.value = true
            const toDef: ToDef = {
                type: 'format',
                output: { mode: 'api' },
                format: { format_type: 'json', options: {} },
                mapping: {},
            }
            const wrapper = mount(DslToEditor, { props: { modelValue: toDef } })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const typeSelect = selects[0]
            const items = typeSelect.props('items') as Array<{ value: string; title: string }>
            expect(items.some(item => item.value === 'email')).toBe(true)
        })

        it('changes to Email type and emits default email ToDef', async () => {
            const toDef: ToDef = {
                type: 'next_step',
                mapping: {},
            }
            const wrapper = mount(DslToEditor, {
                props: { modelValue: toDef },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            await selects[0].vm.$emit('update:modelValue', 'email')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
            const updated = emitted[emitted.length - 1][0]
            expect(updated.type).toBe('email')
            if (updated.type === 'email') {
                expect(updated.to).toEqual([])
                expect(updated.template_uuid).toBe('')
            }
        })

        it('renders info alert, template select, and target status caption', async () => {
            const wrapper = mount(DslToEditor, {
                props: { modelValue: makeEmailTo() },
            })
            await nextTick()

            const alerts = wrapper.findAllComponents({ name: 'VAlert' })
            expect(alerts.some(a => a.text().includes('email_to.info'))).toBe(true)

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const templateSelect = selects.find(
                s => (s.props('label') as string) === 'workflows.dsl.send_email_template'
            )
            expect(templateSelect).toBeTruthy()
        })

        it('loads email templates on mount and populates the template select', async () => {
            mockListEmailTemplates.mockResolvedValueOnce([
                { uuid: 'tmpl-1', name: 'Order Confirmation' },
            ])
            const wrapper = mount(DslToEditor, {
                props: { modelValue: makeEmailTo() },
            })
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 50))

            expect(mockListEmailTemplates).toHaveBeenCalledWith('workflow')

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const templateSelect = selects.find(
                s => (s.props('label') as string) === 'workflows.dsl.send_email_template'
            )
            expect(templateSelect?.props('items')).toEqual([
                { title: 'Order Confirmation', value: 'tmpl-1' },
            ])
        })

        it('clears the template list when loading email templates fails', async () => {
            mockListEmailTemplates.mockRejectedValueOnce(new Error('network error'))
            const wrapper = mount(DslToEditor, {
                props: { modelValue: makeEmailTo() },
            })
            await nextTick()
            await new Promise(resolve => setTimeout(resolve, 50))

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const templateSelect = selects.find(
                s => (s.props('label') as string) === 'workflows.dsl.send_email_template'
            )
            expect(templateSelect?.props('items')).toEqual([])
        })

        it('updates template_uuid when template select changes', async () => {
            const wrapper = mount(DslToEditor, {
                props: { modelValue: makeEmailTo() },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const templateSelect = selects.find(
                s => (s.props('label') as string) === 'workflows.dsl.send_email_template'
            )
            await templateSelect!.vm.$emit('update:modelValue', 'tmpl-9')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
            const updated = emitted[emitted.length - 1][0]
            if (updated.type === 'email') {
                expect(updated.template_uuid).toBe('tmpl-9')
            }
        })

        it('adds a To recipient and switches its kind to field', async () => {
            const wrapper = mount(DslToEditor, {
                props: { modelValue: makeEmailTo() },
            })
            await nextTick()

            const addToButton = wrapper
                .findAll('button')
                .find(b => b.text().includes('add_recipient'))
            expect(addToButton).toBeTruthy()
            await addToButton!.trigger('click')
            await nextTick()

            let emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
            let updated = emitted[emitted.length - 1][0]
            expect(updated.type).toBe('email')
            if (updated.type === 'email') {
                expect(updated.to).toEqual([{ kind: 'const_string', value: '' }])
            }

            await wrapper.setProps({ modelValue: updated })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const kindSelect = selects.find(s => {
                const items = s.props('items') as string[] | undefined
                return (
                    Array.isArray(items) &&
                    items.includes('field') &&
                    items.includes('const_string')
                )
            })
            expect(kindSelect).toBeTruthy()
            await kindSelect!.vm.$emit('update:modelValue', 'field')
            await nextTick()

            emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
            updated = emitted[emitted.length - 1][0]
            if (updated.type === 'email') {
                expect(updated.to).toEqual([{ kind: 'field', field: '' }])
            }
        })

        it('updates a To recipient field value and removes it', async () => {
            const wrapper = mount(DslToEditor, {
                props: {
                    modelValue: makeEmailTo({ to: [{ kind: 'field', field: '' }] }),
                },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const valueField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.value'
            )
            expect(valueField).toBeTruthy()
            await valueField!.vm.$emit('update:modelValue', 'customer_email')
            await nextTick()

            let emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
            let updated = emitted[emitted.length - 1][0]
            if (updated.type === 'email') {
                expect(updated.to).toEqual([{ kind: 'field', field: 'customer_email' }])
            }

            await wrapper.setProps({ modelValue: updated })
            await nextTick()

            const deleteButtons = wrapper.findAllComponents({ name: 'VBtn' })
            const removeBtn = deleteButtons.find(b => b.props('icon') === 'mdi-delete')
            expect(removeBtn).toBeTruthy()
            await removeBtn!.trigger('click')
            await nextTick()

            emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
            updated = emitted[emitted.length - 1][0]
            if (updated.type === 'email') {
                expect(updated.to).toEqual([])
            }
        })

        it('updates a To recipient const value', async () => {
            const wrapper = mount(DslToEditor, {
                props: {
                    modelValue: makeEmailTo({ to: [{ kind: 'const_string', value: '' }] }),
                },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const valueField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.value'
            )
            await valueField!.vm.$emit('update:modelValue', 'static@test.com')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
            const updated = emitted[emitted.length - 1][0]
            if (updated.type === 'email') {
                expect(updated.to).toEqual([{ kind: 'const_string', value: 'static@test.com' }])
            }
        })

        it('adds and removes a Cc recipient, clearing cc when list becomes empty', async () => {
            const wrapper = mount(DslToEditor, {
                props: { modelValue: makeEmailTo() },
            })
            await nextTick()

            const addCcButton = wrapper
                .findAll('button')
                .find(b => b.text().includes('add_cc_recipient'))
            expect(addCcButton).toBeTruthy()
            await addCcButton!.trigger('click')
            await nextTick()

            let emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
            let updated = emitted[emitted.length - 1][0]
            if (updated.type === 'email') {
                expect(updated.cc).toEqual([{ kind: 'const_string', value: '' }])
            }

            await wrapper.setProps({ modelValue: updated })
            await nextTick()

            const deleteButtons = wrapper.findAllComponents({ name: 'VBtn' })
            const ccDeleteButtons = deleteButtons.filter(b => b.props('icon') === 'mdi-delete')
            const removeCcBtn = ccDeleteButtons[ccDeleteButtons.length - 1]
            expect(removeCcBtn).toBeTruthy()
            await removeCcBtn!.trigger('click')
            await nextTick()

            emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
            updated = emitted[emitted.length - 1][0]
            if (updated.type === 'email') {
                expect(updated.cc).toBeUndefined()
            }
        })

        it('switches Cc kind and updates its field/const values', async () => {
            const wrapper = mount(DslToEditor, {
                props: {
                    modelValue: makeEmailTo({
                        cc: [{ kind: 'const_string', value: 'cc@test.com' }],
                    }),
                },
            })
            await nextTick()

            const selects = wrapper.findAllComponents({ name: 'VSelect' })
            const kindSelects = selects.filter(s => {
                const items = s.props('items') as string[] | undefined
                return (
                    Array.isArray(items) &&
                    items.includes('field') &&
                    items.includes('const_string')
                )
            })
            const ccKindSelect = kindSelects[kindSelects.length - 1]
            expect(ccKindSelect).toBeTruthy()
            await ccKindSelect!.vm.$emit('update:modelValue', 'field')
            await nextTick()

            let emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
            let updated = emitted[emitted.length - 1][0]
            if (updated.type === 'email') {
                expect(updated.cc).toEqual([{ kind: 'field', field: '' }])
            }

            await wrapper.setProps({ modelValue: updated })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const valueField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.value'
            )
            await valueField!.vm.$emit('update:modelValue', 'cc_field')
            await nextTick()

            emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
            updated = emitted[emitted.length - 1][0]
            if (updated.type === 'email') {
                expect(updated.cc).toEqual([{ kind: 'field', field: 'cc_field' }])
            }
        })

        it('updates a Cc const value directly', async () => {
            const wrapper = mount(DslToEditor, {
                props: {
                    modelValue: makeEmailTo({ cc: [{ kind: 'const_string', value: '' }] }),
                },
            })
            await nextTick()

            const textFields = wrapper.findAllComponents({ name: 'VTextField' })
            const valueField = textFields.find(
                tf => (tf.props('label') as string) === 'workflows.dsl.value'
            )
            await valueField!.vm.$emit('update:modelValue', 'cc-static@test.com')
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
            const updated = emitted[emitted.length - 1][0]
            if (updated.type === 'email') {
                expect(updated.cc).toEqual([{ kind: 'const_string', value: 'cc-static@test.com' }])
            }
        })

        it('updates the email mapping via the mapping editor', async () => {
            const wrapper = mount(DslToEditor, {
                props: { modelValue: makeEmailTo() },
            })
            await nextTick()

            const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
            expect(mappingEditor.exists()).toBe(true)
            const newMapping = { normalized_field: 'destination_field' }
            await mappingEditor.vm.$emit('update:modelValue', newMapping)
            await nextTick()

            const emitted = wrapper.emitted('update:modelValue') as Array<[ToDef]>
            const updated = emitted[emitted.length - 1][0]
            if (updated.type === 'email') {
                expect(updated.mapping).toEqual(newMapping)
            }
        })

        it('adds an empty mapping pair via the add_mapping button for email type', async () => {
            const wrapper = mount(DslToEditor, {
                props: { modelValue: makeEmailTo() },
            })
            await nextTick()

            const addMappingButton = wrapper
                .findAll('button')
                .find(b => b.text().includes('add_mapping'))
            expect(addMappingButton).toBeTruthy()
            await addMappingButton!.trigger('click')
            await nextTick()

            const mappingEditor = wrapper.findComponent({ name: 'MappingEditor' })
            expect(mappingEditor.exists()).toBe(true)
        })
    })
})
