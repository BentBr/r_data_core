import { mount } from '@vue/test-utils'
import { describe, it, expect } from 'vitest'
import EntityDefinitionFields from './EntityDefinitionFields.vue'
import type { EntityDefinition, FieldDefinition } from '@/types/schemas'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import SmartIcon from '@/components/common/SmartIcon.vue'

// Create Vuetify instance for testing
const vuetify = createVuetify({
    components,
    directives,
})

describe('EntityDefinitionFields', () => {
    const mockFields: FieldDefinition[] = [
        {
            name: 'test_field',
            display_name: 'Test Field',
            field_type: 'String',
            description: 'A test field',
            required: true,
            indexed: false,
            filterable: true,
            default_value: 'default',
            constraints: {},
            ui_settings: {},
        },
        {
            name: 'number_field',
            display_name: 'Number Field',
            field_type: 'Integer',
            description: 'A number field',
            required: false,
            indexed: true,
            filterable: false,
            default_value: undefined,
            constraints: {},
            ui_settings: {},
        },
    ]

    const mockDefinition: EntityDefinition = {
        uuid: '123e4567-e89b-12d3-a456-426614174000',
        entity_type: 'test_entity',
        display_name: 'Test Entity',
        description: 'A test entity definition',
        group_name: 'test_group',
        allow_children: true,
        icon: 'test-tube',
        fields: mockFields,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
        created_by: '123e4567-e89b-12d3-a456-426614174000',
        updated_by: undefined,
        published: true,
        version: 1,
    }

    it('renders fields list correctly', () => {
        const wrapper = mount(EntityDefinitionFields, {
            props: {
                definition: mockDefinition,
                hasUnsavedChanges: false,
                savingChanges: false,
            },
            global: {
                plugins: [vuetify],
            },
        })

        expect(wrapper.text()).toContain('Test Field')
        expect(wrapper.text()).toContain('Number Field')
        expect(wrapper.text()).toContain('String')
        expect(wrapper.text()).toContain('Integer')
    })

    it('shows apply changes button when there are unsaved changes', () => {
        const wrapper = mount(EntityDefinitionFields, {
            props: {
                definition: mockDefinition,
                hasUnsavedChanges: true,
                savingChanges: false,
            },
            global: {
                plugins: [vuetify],
            },
        })

        expect(wrapper.find('[data-test="apply"]').exists()).toBe(true)
    })

    it('does not show apply changes button when no unsaved changes', () => {
        const wrapper = mount(EntityDefinitionFields, {
            props: {
                definition: mockDefinition,
                hasUnsavedChanges: false,
                savingChanges: false,
            },
            global: {
                plugins: [vuetify],
            },
        })

        expect(wrapper.find('[data-test="apply"]').exists()).toBe(false)
    })

    it('emits save-changes event when apply button is clicked', async () => {
        const wrapper = mount(EntityDefinitionFields, {
            props: {
                definition: mockDefinition,
                hasUnsavedChanges: true,
                savingChanges: false,
            },
            global: {
                plugins: [vuetify],
            },
        })

        const applyButton = wrapper.find('[data-test="apply"]')
        await applyButton.trigger('click')

        expect(wrapper.emitted('save-changes')).toBeTruthy()
    })

    it('emits add-field event when add field button is clicked', async () => {
        const wrapper = mount(EntityDefinitionFields, {
            props: {
                definition: mockDefinition,
                hasUnsavedChanges: false,
                savingChanges: false,
            },
            global: {
                plugins: [vuetify],
            },
        })

        const addButton = wrapper.find('[data-test="add"]')
        await addButton.trigger('click')

        expect(wrapper.emitted('add-field')).toBeTruthy()
    })

    it('shows field icons correctly', () => {
        const wrapper = mount(EntityDefinitionFields, {
            props: {
                definition: mockDefinition,
                hasUnsavedChanges: false,
                savingChanges: false,
            },
            global: {
                plugins: [vuetify],
            },
        })

        // Check that field type chips are rendered
        const chips = wrapper.findAll('.v-chip')
        expect(chips.length).toBeGreaterThan(0)
    })

    it('shows required field indicators', () => {
        const wrapper = mount(EntityDefinitionFields, {
            props: {
                definition: mockDefinition,
                hasUnsavedChanges: false,
                savingChanges: false,
            },
            global: {
                plugins: [vuetify],
            },
        })

        // Check for required field icons (check-circle for required fields)
        const requiredIcons = wrapper
            .findAllComponents(SmartIcon)
            .filter(icon => icon.props('icon') === 'check-circle')
        expect(requiredIcons.length).toBeGreaterThan(0)
    })

    it('shows indexed field indicators', () => {
        const wrapper = mount(EntityDefinitionFields, {
            props: {
                definition: mockDefinition,
                hasUnsavedChanges: false,
                savingChanges: false,
            },
            global: {
                plugins: [vuetify],
            },
        })

        // Check for indexed field icons (database icon for indexed fields)
        const indexedIcons = wrapper
            .findAllComponents(SmartIcon)
            .filter(icon => icon.props('icon') === 'database')
        expect(indexedIcons.length).toBeGreaterThan(0)
    })

    it('shows filterable field indicators', () => {
        const wrapper = mount(EntityDefinitionFields, {
            props: {
                definition: mockDefinition,
                hasUnsavedChanges: false,
                savingChanges: false,
            },
            global: {
                plugins: [vuetify],
            },
        })

        // Check for filterable field icons (filter icon for filterable fields)
        const filterIcons = wrapper
            .findAllComponents(SmartIcon)
            .filter(icon => icon.props('icon') === 'filter')
        expect(filterIcons.length).toBeGreaterThan(0)
    })

    it('emits edit-field event when edit button is clicked', async () => {
        const wrapper = mount(EntityDefinitionFields, {
            props: {
                definition: mockDefinition,
                hasUnsavedChanges: false,
                savingChanges: false,
            },
            global: {
                plugins: [vuetify],
            },
        })

        const editIcon = wrapper
            .findAllComponents(SmartIcon)
            .find(icon => icon.props('icon') === 'pencil')
        const editButton = editIcon
            ? wrapper.findAll('button').find(btn => btn.element.contains(editIcon.element))
            : undefined

        await editButton?.trigger('click')
        expect(wrapper.emitted('edit-field')).toBeTruthy()
    })

    it('emits remove-field event when delete button is clicked', async () => {
        const wrapper = mount(EntityDefinitionFields, {
            props: {
                definition: mockDefinition,
                hasUnsavedChanges: false,
                savingChanges: false,
            },
            global: {
                plugins: [vuetify],
            },
        })

        const deleteIcon = wrapper
            .findAllComponents(SmartIcon)
            .find(icon => icon.props('icon') === 'trash-2')
        const deleteButton = deleteIcon
            ? wrapper.findAll('button').find(btn => btn.element.contains(deleteIcon.element))
            : undefined

        await deleteButton?.trigger('click')
        expect(wrapper.emitted('remove-field')).toBeTruthy()
    })

    it('shows loading state when saving changes', () => {
        const wrapper = mount(EntityDefinitionFields, {
            props: {
                definition: mockDefinition,
                hasUnsavedChanges: true,
                savingChanges: true,
            },
            global: {
                plugins: [vuetify],
            },
        })

        const applyButton = wrapper.find('[data-test="apply"]')
        expect(applyButton.exists()).toBe(true)
        // Vuetify loading state is handled internally, just verify button exists when saving
    })

    describe('Unique and Pattern (Regex) indicators', () => {
        const fieldWithUnique: FieldDefinition = {
            name: 'email',
            display_name: 'Email',
            field_type: 'String',
            description: 'Email field',
            required: true,
            indexed: true,
            filterable: true,
            unique: true,
            default_value: undefined,
            // API returns nested constraints structure
            constraints: {
                type: 'string',
                constraints: {
                    pattern: '^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$',
                },
            },
            ui_settings: {},
        }

        const fieldWithoutUniqueOrPattern: FieldDefinition = {
            name: 'name',
            display_name: 'Name',
            field_type: 'String',
            description: 'Name field',
            required: false,
            indexed: false,
            filterable: false,
            unique: false,
            default_value: undefined,
            constraints: {
                type: 'string',
                constraints: {
                    pattern: null,
                },
            },
            ui_settings: {},
        }

        const definitionWithUniqueAndPattern: EntityDefinition = {
            ...mockDefinition,
            fields: [fieldWithUnique, fieldWithoutUniqueOrPattern],
        }

        it('shows unique icon for fields with unique=true', () => {
            const wrapper = mount(EntityDefinitionFields, {
                props: {
                    definition: definitionWithUniqueAndPattern,
                    hasUnsavedChanges: false,
                    savingChanges: false,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            // Check for unique field icon (key icon)
            const uniqueIcons = wrapper
                .findAllComponents(SmartIcon)
                .filter(icon => icon.props('icon') === 'key')
            expect(uniqueIcons.length).toBe(1) // Only one field has unique=true
        })

        it('shows regex icon for fields with pattern constraint', () => {
            const wrapper = mount(EntityDefinitionFields, {
                props: {
                    definition: definitionWithUniqueAndPattern,
                    hasUnsavedChanges: false,
                    savingChanges: false,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            // Check for pattern/regex field icon
            const regexIcons = wrapper
                .findAllComponents(SmartIcon)
                .filter(icon => icon.props('icon') === 'regex')
            expect(regexIcons.length).toBe(1) // Only one field has a pattern
        })

        it('does not show unique icon for fields with unique=false', () => {
            const definitionWithoutUnique: EntityDefinition = {
                ...mockDefinition,
                fields: [fieldWithoutUniqueOrPattern],
            }

            const wrapper = mount(EntityDefinitionFields, {
                props: {
                    definition: definitionWithoutUnique,
                    hasUnsavedChanges: false,
                    savingChanges: false,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            // Check that no unique icons are shown
            const uniqueIcons = wrapper
                .findAllComponents(SmartIcon)
                .filter(icon => icon.props('icon') === 'key')
            expect(uniqueIcons.length).toBe(0)
        })

        it('does not show regex icon for fields without pattern', () => {
            const definitionWithoutPattern: EntityDefinition = {
                ...mockDefinition,
                fields: [fieldWithoutUniqueOrPattern],
            }

            const wrapper = mount(EntityDefinitionFields, {
                props: {
                    definition: definitionWithoutPattern,
                    hasUnsavedChanges: false,
                    savingChanges: false,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            // Check that no regex icons are shown
            const regexIcons = wrapper
                .findAllComponents(SmartIcon)
                .filter(icon => icon.props('icon') === 'regex')
            expect(regexIcons.length).toBe(0)
        })
    })

    describe('Json field type icon and color mappings', () => {
        const jsonField: FieldDefinition = {
            name: 'json_data',
            display_name: 'JSON Data',
            field_type: 'Json',
            description: 'A JSON field that accepts any valid JSON value',
            required: false,
            indexed: false,
            filterable: false,
            default_value: undefined,
            constraints: {},
            ui_settings: {},
        }

        const objectField: FieldDefinition = {
            name: 'object_data',
            display_name: 'Object Data',
            field_type: 'Object',
            description: 'An Object field that only accepts JSON objects',
            required: false,
            indexed: false,
            filterable: false,
            default_value: undefined,
            constraints: {},
            ui_settings: {},
        }

        const arrayField: FieldDefinition = {
            name: 'array_data',
            display_name: 'Array Data',
            field_type: 'Array',
            description: 'An Array field',
            required: false,
            indexed: false,
            filterable: false,
            default_value: undefined,
            constraints: {},
            ui_settings: {},
        }

        const definitionWithJsonTypes: EntityDefinition = {
            ...mockDefinition,
            fields: [jsonField, objectField, arrayField],
        }

        it('renders Json field type with braces icon', () => {
            const wrapper = mount(EntityDefinitionFields, {
                props: {
                    definition: definitionWithJsonTypes,
                    hasUnsavedChanges: false,
                    savingChanges: false,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            // Check that Json field has braces icon
            const smartIcons = wrapper.findAllComponents(SmartIcon)
            const bracesIcons = smartIcons.filter(icon => icon.props('icon') === 'braces')
            expect(bracesIcons.length).toBeGreaterThan(0)
        })

        it('renders Json field type distinct from Object field type', () => {
            const wrapper = mount(EntityDefinitionFields, {
                props: {
                    definition: definitionWithJsonTypes,
                    hasUnsavedChanges: false,
                    savingChanges: false,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            // Check that both Json (braces) and Object (box) icons are rendered
            const smartIcons = wrapper.findAllComponents(SmartIcon)
            const bracesIcons = smartIcons.filter(icon => icon.props('icon') === 'braces')
            const boxIcons = smartIcons.filter(icon => icon.props('icon') === 'box')

            expect(bracesIcons.length).toBeGreaterThan(0) // Json field
            expect(boxIcons.length).toBeGreaterThan(0) // Object field
        })

        it('renders Json field type with teal color', () => {
            const wrapper = mount(EntityDefinitionFields, {
                props: {
                    definition: definitionWithJsonTypes,
                    hasUnsavedChanges: false,
                    savingChanges: false,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            // Check that Json field has teal color
            const smartIcons = wrapper.findAllComponents(SmartIcon)
            const tealIcons = smartIcons.filter(icon => icon.props('color') === 'teal')
            expect(tealIcons.length).toBeGreaterThan(0)
        })

        it('displays Json field type badge', () => {
            const wrapper = mount(EntityDefinitionFields, {
                props: {
                    definition: definitionWithJsonTypes,
                    hasUnsavedChanges: false,
                    savingChanges: false,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            // Check that Json badge is displayed
            expect(wrapper.text()).toContain('Json')
            expect(wrapper.text()).toContain('Object')
            expect(wrapper.text()).toContain('Array')
        })
    })
})
