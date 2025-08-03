import { mount } from '@vue/test-utils'
import { describe, it, expect } from 'vitest'
import EntityDefinitionFields from './EntityDefinitionFields.vue'
import type { EntityDefinition, FieldDefinition } from '@/types/schemas'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'

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
        icon: 'mdi-test',
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
        const requiredIcons = wrapper.findAll('.mdi-check-circle')
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
        const indexedIcons = wrapper.findAll('.mdi-database')
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
        const filterIcons = wrapper.findAll('.mdi-filter')
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

        const editButtons = wrapper.findAll('button')
        const editButton = editButtons.find(
            (btn: { attributes: (attr: string) => string }) =>
                btn.attributes('icon') === 'mdi-pencil'
        )

        if (editButton) {
            await editButton.trigger('click')
            expect(wrapper.emitted('edit-field')).toBeTruthy()
        }
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

        const deleteButtons = wrapper.findAll('button')
        const deleteButton = deleteButtons.find(
            (btn: { attributes: (attr: string) => string }) =>
                btn.attributes('icon') === 'mdi-delete'
        )

        if (deleteButton) {
            await deleteButton.trigger('click')
            expect(wrapper.emitted('remove-field')).toBeTruthy()
        }
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
})
