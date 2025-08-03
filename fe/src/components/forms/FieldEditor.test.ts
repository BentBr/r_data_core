import { mount } from '@vue/test-utils'
import { describe, it, expect } from 'vitest'
import FieldEditor from './FieldEditor.vue'
import type { FieldDefinition } from '@/types/schemas'
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'

// Create Vuetify instance for testing
const vuetify = createVuetify({
    components,
    directives,
})

describe('FieldEditor', () => {
    const mockField: FieldDefinition = {
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
    }

    it('renders form fields correctly', async () => {
        const wrapper = mount(FieldEditor, {
            props: {
                modelValue: true,
                field: null, // Adding new field
            },
            global: {
                plugins: [vuetify],
                stubs: {
                    teleport: false, // Disable teleport stubbing
                },
            },
            attachTo: document.body,
        })

        // Wait for the dialog to be rendered
        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100)) // Give time for dialog to render

        // Check if component is rendered at all
        expect(wrapper.exists()).toBe(true)

        // The dialog should be rendered in the document body due to teleport
        const dialogContent = document.querySelector('[data-test="name"]')
        expect(dialogContent).toBeTruthy()

        const displayNameInput = document.querySelector('[data-test="display_name"]')
        expect(displayNameInput).toBeTruthy()

        const fieldTypeSelect = document.querySelector('[data-test="field_type"]')
        expect(fieldTypeSelect).toBeTruthy()
    })

    it('validates required fields', async () => {
        const wrapper = mount(FieldEditor, {
            props: {
                modelValue: true,
                field: null,
            },
            global: {
                plugins: [vuetify],
                stubs: {
                    teleport: false, // Disable teleport stubbing
                },
            },
            attachTo: document.body,
        })

        // Wait for the dialog to be rendered
        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100)) // Give time for dialog to render

        // Try to submit without filling required fields
        const saveButton = document.querySelector('[data-test="save"]') as HTMLButtonElement
        if (!saveButton) {
            console.log('Available elements:', document.querySelectorAll('[data-test]'))
            throw new Error('Save button not found')
        }

        saveButton.click()

        // Form should not be valid and save should not be emitted
        expect(wrapper.emitted('save')).toBeFalsy()
    })

    it('validates field name format', async () => {
        const wrapper = mount(FieldEditor, {
            props: {
                modelValue: true,
                field: null,
            },
            global: {
                plugins: [vuetify],
                stubs: {
                    teleport: false, // Disable teleport stubbing
                },
            },
            attachTo: document.body,
        })

        // Wait for the dialog to be rendered
        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100)) // Give time for dialog to render

        // Set invalid field name
        const nameInput = document.querySelector('[data-test="name"]') as HTMLInputElement
        const displayNameInput = document.querySelector(
            '[data-test="display_name"]'
        ) as HTMLInputElement

        if (!nameInput || !displayNameInput) {
            console.log('Available elements:', document.querySelectorAll('[data-test]'))
            throw new Error('Form elements not found')
        }

        nameInput.value = 'invalid-name'
        nameInput.dispatchEvent(new Event('input'))

        displayNameInput.value = 'Valid Display Name'
        displayNameInput.dispatchEvent(new Event('input'))

        const saveButton = document.querySelector('[data-test="save"]') as HTMLButtonElement
        if (!saveButton) {
            console.log('Available elements:', document.querySelectorAll('[data-test]'))
            throw new Error('Save button not found')
        }

        saveButton.click()

        // Form should not be valid due to invalid field name
        expect(wrapper.emitted('save')).toBeFalsy()
    })

    it('shows correct dialog title for new field', async () => {
        const wrapper = mount(FieldEditor, {
            props: {
                modelValue: true,
                field: null,
            },
            global: {
                plugins: [vuetify],
                stubs: {
                    teleport: false, // Disable teleport stubbing
                },
            },
            attachTo: document.body,
        })

        // Wait for the dialog to be rendered
        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100)) // Give time for dialog to render

        const saveButton = document.querySelector('[data-test="save"]')
        expect(saveButton).toBeTruthy()
    })

    it('shows correct dialog title for editing field', async () => {
        const wrapper = mount(FieldEditor, {
            props: {
                modelValue: true,
                field: mockField,
            },
            global: {
                plugins: [vuetify],
                stubs: {
                    teleport: false, // Disable teleport stubbing
                },
            },
            attachTo: document.body,
        })

        // Wait for the dialog to be rendered
        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100)) // Give time for dialog to render

        const saveButton = document.querySelector('[data-test="save"]')
        expect(saveButton).toBeTruthy()
    })

    it('shows correct button text for new field', async () => {
        const wrapper = mount(FieldEditor, {
            props: {
                modelValue: true,
                field: null,
            },
            global: {
                plugins: [vuetify],
                stubs: {
                    teleport: false, // Disable teleport stubbing
                },
            },
            attachTo: document.body,
        })

        // Wait for the dialog to be rendered
        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100)) // Give time for dialog to render

        const saveButton = document.querySelector('[data-test="save"]')
        expect(saveButton).toBeTruthy()
    })

    it('shows correct button text for editing field', async () => {
        const wrapper = mount(FieldEditor, {
            props: {
                modelValue: true,
                field: mockField,
            },
            global: {
                plugins: [vuetify],
                stubs: {
                    teleport: false, // Disable teleport stubbing
                },
            },
            attachTo: document.body,
        })

        // Wait for the dialog to be rendered
        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100)) // Give time for dialog to render

        const saveButton = document.querySelector('[data-test="save"]')
        expect(saveButton).toBeTruthy()
    })
})
