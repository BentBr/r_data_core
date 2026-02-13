import { mount } from '@vue/test-utils'
import { describe, it, expect, vi } from 'vitest'
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

// Mock translations
vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({
        t: (key: string) => key,
    }),
}))

describe('FieldEditor', () => {
    const EMAIL_REGEX = '^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$'

    describe('Loading field with nested constraints from API', () => {
        it('should extract pattern from nested constraints structure', async () => {
            // Field as returned by API with nested constraints
            const fieldFromApi: FieldDefinition = {
                name: 'email',
                display_name: 'Email',
                field_type: 'String',
                description: '',
                required: true,
                indexed: true,
                filterable: true,
                unique: true,
                default_value: undefined,
                constraints: {
                    type: 'string',
                    constraints: {
                        pattern: EMAIL_REGEX,
                        min_length: null,
                        max_length: null,
                    },
                },
                ui_settings: {},
            }

            const wrapper = mount(FieldEditor, {
                props: {
                    modelValue: true,
                    field: fieldFromApi,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            await wrapper.vm.$nextTick()

            // Access exposed properties
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                constraintPattern: string | undefined
                emailPreset: boolean
                constraintUnique: boolean
            }

            // The form should have extracted inner constraints (flat)
            expect(vm.form.constraints).toBeDefined()
            expect(vm.form.constraints?.pattern).toBe(EMAIL_REGEX)

            // constraintPattern computed should return the pattern
            expect(vm.constraintPattern).toBe(EMAIL_REGEX)

            // emailPreset should be true since pattern matches EMAIL_REGEX
            expect(vm.emailPreset).toBe(true)

            // unique should be loaded from field
            expect(vm.constraintUnique).toBe(true)
        })

        it('should handle field without constraints gracefully', async () => {
            const fieldWithoutConstraints: FieldDefinition = {
                name: 'name',
                display_name: 'Name',
                field_type: 'String',
                description: '',
                required: false,
                indexed: false,
                filterable: false,
                unique: false,
                default_value: undefined,
                constraints: undefined,
                ui_settings: {},
            }

            const wrapper = mount(FieldEditor, {
                props: {
                    modelValue: true,
                    field: fieldWithoutConstraints,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                constraintPattern: string | undefined
                emailPreset: boolean
            }

            // Form should have empty constraints
            expect(vm.constraintPattern).toBeUndefined()
            expect(vm.emailPreset).toBe(false)
        })

        it('should extract min_length and max_length from nested constraints', async () => {
            const fieldWithLengthConstraints: FieldDefinition = {
                name: 'username',
                display_name: 'Username',
                field_type: 'String',
                description: '',
                required: true,
                indexed: false,
                filterable: false,
                unique: false,
                default_value: undefined,
                constraints: {
                    type: 'string',
                    constraints: {
                        min_length: 3,
                        max_length: 50,
                        pattern: null,
                    },
                },
                ui_settings: {},
            }

            const wrapper = mount(FieldEditor, {
                props: {
                    modelValue: true,
                    field: fieldWithLengthConstraints,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                constraintMinLength: number | undefined
                constraintMaxLength: number | undefined
            }

            expect(vm.constraintMinLength).toBe(3)
            expect(vm.constraintMaxLength).toBe(50)
        })

        it('should format constraints back to nested structure on save', async () => {
            const wrapper = mount(FieldEditor, {
                props: {
                    modelValue: true,
                    field: undefined, // New field
                },
                global: {
                    plugins: [vuetify],
                },
            })

            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }

            // Set form values
            vm.form.name = 'test'
            vm.form.display_name = 'Test'
            vm.form.field_type = 'String'
            vm.form.constraints = { pattern: '^test$' }
            vm.formValid = true

            // Save the field
            vm.saveField()

            // Check emitted event
            const saveEvents = wrapper.emitted('save')
            expect(saveEvents).toBeTruthy()
            expect(saveEvents?.length).toBe(1)

            const savedField = saveEvents?.[0]?.[0] as FieldDefinition
            // Should have nested structure
            expect(savedField.constraints).toEqual({
                type: 'string',
                constraints: { pattern: '^test$' },
            })
        })
    })

    describe('Numeric field constraints', () => {
        it('should extract min and max values from nested constraints for Integer fields', async () => {
            const fieldWithNumericConstraints: FieldDefinition = {
                name: 'quantity',
                display_name: 'Quantity',
                field_type: 'Integer',
                description: '',
                required: true,
                indexed: false,
                filterable: false,
                unique: false,
                default_value: undefined,
                constraints: {
                    type: 'integer',
                    constraints: {
                        min: 0,
                        max: 1000,
                        positive_only: true,
                    },
                },
                ui_settings: {},
            }

            const wrapper = mount(FieldEditor, {
                props: {
                    modelValue: true,
                    field: fieldWithNumericConstraints,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                constraintMin: number | undefined
                constraintMax: number | undefined
                constraintPositiveOnly: boolean | undefined
            }

            expect(vm.constraintMin).toBe(0)
            expect(vm.constraintMax).toBe(1000)
            expect(vm.constraintPositiveOnly).toBe(true)
        })

        it('should extract min and max values from nested constraints for Float fields', async () => {
            const fieldWithFloatConstraints: FieldDefinition = {
                name: 'price',
                display_name: 'Price',
                field_type: 'Float',
                description: '',
                required: true,
                indexed: true,
                filterable: true,
                unique: false,
                default_value: undefined,
                constraints: {
                    type: 'float',
                    constraints: {
                        min: 0.01,
                        max: 99999.99,
                    },
                },
                ui_settings: {},
            }

            const wrapper = mount(FieldEditor, {
                props: {
                    modelValue: true,
                    field: fieldWithFloatConstraints,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                constraintMin: number | undefined
                constraintMax: number | undefined
            }

            expect(vm.constraintMin).toBe(0.01)
            expect(vm.constraintMax).toBe(99999.99)
        })

        it('should format numeric constraints back to nested structure on save', async () => {
            const wrapper = mount(FieldEditor, {
                props: {
                    modelValue: true,
                    field: undefined,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }

            // Set form values for numeric field
            vm.form.name = 'quantity'
            vm.form.display_name = 'Quantity'
            vm.form.field_type = 'Integer'
            vm.form.constraints = { min: 0, max: 100 }
            vm.formValid = true

            // Save the field
            vm.saveField()

            // Check emitted event
            const saveEvents = wrapper.emitted('save')
            expect(saveEvents).toBeTruthy()

            const savedField = saveEvents?.[0]?.[0] as FieldDefinition
            // Should have nested structure with integer type
            expect(savedField.constraints).toEqual({
                type: 'integer',
                constraints: { min: 0, max: 100 },
            })
        })
    })

    describe('Unique field handling', () => {
        it('should preserve unique=true through load and save cycle', async () => {
            const fieldWithUnique: FieldDefinition = {
                name: 'email',
                display_name: 'Email',
                field_type: 'String',
                description: '',
                required: true,
                indexed: true,
                filterable: true,
                unique: true,
                default_value: undefined,
                constraints: {},
                ui_settings: {},
            }

            const wrapper = mount(FieldEditor, {
                props: {
                    modelValue: true,
                    field: fieldWithUnique,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                constraintUnique: boolean
                formValid: boolean
                saveField: () => void
            }

            // Verify unique is loaded
            expect(vm.constraintUnique).toBe(true)
            expect(vm.form.unique).toBe(true)

            // Save without changes
            vm.formValid = true
            vm.saveField()

            const saveEvents = wrapper.emitted('save')
            expect(saveEvents).toBeTruthy()

            const savedField = saveEvents?.[0]?.[0] as FieldDefinition
            expect(savedField.unique).toBe(true)
        })

        it('should preserve unique=false through load and save cycle', async () => {
            const fieldWithoutUnique: FieldDefinition = {
                name: 'name',
                display_name: 'Name',
                field_type: 'String',
                description: '',
                required: true,
                indexed: false,
                filterable: false,
                unique: false,
                default_value: undefined,
                constraints: {},
                ui_settings: {},
            }

            const wrapper = mount(FieldEditor, {
                props: {
                    modelValue: true,
                    field: fieldWithoutUnique,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                constraintUnique: boolean
                formValid: boolean
                saveField: () => void
            }

            // Verify unique is loaded as false
            expect(vm.constraintUnique).toBe(false)
            expect(vm.form.unique).toBe(false)

            // Save without changes
            vm.formValid = true
            vm.saveField()

            const saveEvents = wrapper.emitted('save')
            const savedField = saveEvents?.[0]?.[0] as FieldDefinition
            expect(savedField.unique).toBe(false)
        })

        it('should handle toggling unique on for a new field', async () => {
            const wrapper = mount(FieldEditor, {
                props: {
                    modelValue: true,
                    field: undefined,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }

            // Set form values with unique=true
            vm.form.name = 'email'
            vm.form.display_name = 'Email'
            vm.form.field_type = 'String'
            vm.form.unique = true
            vm.formValid = true

            // Save the field
            vm.saveField()

            const saveEvents = wrapper.emitted('save')
            const savedField = saveEvents?.[0]?.[0] as FieldDefinition
            expect(savedField.unique).toBe(true)
        })
    })

    describe('Password field type', () => {
        it('should show string validation options for Password fields', async () => {
            const wrapper = mount(FieldEditor, {
                props: {
                    modelValue: true,
                    field: undefined,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                isStringType: boolean
                showDefaultValue: boolean
                showValidationSection: boolean
            }

            // Set field type to Password
            vm.form.field_type = 'Password'
            await wrapper.vm.$nextTick()

            // Password should be treated as a string type for validation
            expect(vm.isStringType).toBe(true)
            expect(vm.showValidationSection).toBe(true)
            // Password fields should not show default value
            expect(vm.showDefaultValue).toBe(false)
        })

        it('should format Password constraints back to string type on save', async () => {
            const wrapper = mount(FieldEditor, {
                props: {
                    modelValue: true,
                    field: undefined,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }

            vm.form.name = 'password'
            vm.form.display_name = 'Password'
            vm.form.field_type = 'Password'
            vm.form.constraints = { min_length: 8 }
            vm.formValid = true

            vm.saveField()

            const saveEvents = wrapper.emitted('save')
            expect(saveEvents).toBeTruthy()

            const savedField = saveEvents?.[0]?.[0] as FieldDefinition
            expect(savedField.constraints).toEqual({
                type: 'string',
                constraints: { min_length: 8 },
            })
        })
    })

    describe('Combined constraints', () => {
        it('should handle field with both unique and pattern constraints', async () => {
            const EMAIL_REGEX = '^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$'

            const fieldWithCombinedConstraints: FieldDefinition = {
                name: 'email',
                display_name: 'Email',
                field_type: 'String',
                description: '',
                required: true,
                indexed: true,
                filterable: true,
                unique: true,
                default_value: undefined,
                constraints: {
                    type: 'string',
                    constraints: {
                        pattern: EMAIL_REGEX,
                        min_length: 5,
                        max_length: 255,
                    },
                },
                ui_settings: {},
            }

            const wrapper = mount(FieldEditor, {
                props: {
                    modelValue: true,
                    field: fieldWithCombinedConstraints,
                },
                global: {
                    plugins: [vuetify],
                },
            })

            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                constraintUnique: boolean
                constraintPattern: string | undefined
                constraintMinLength: number | undefined
                constraintMaxLength: number | undefined
                emailPreset: boolean
                formValid: boolean
                saveField: () => void
            }

            // Verify all constraints are loaded
            expect(vm.constraintUnique).toBe(true)
            expect(vm.constraintPattern).toBe(EMAIL_REGEX)
            expect(vm.constraintMinLength).toBe(5)
            expect(vm.constraintMaxLength).toBe(255)
            expect(vm.emailPreset).toBe(true)

            // Save and verify all constraints are preserved
            vm.formValid = true
            vm.saveField()

            const saveEvents = wrapper.emitted('save')
            const savedField = saveEvents?.[0]?.[0] as FieldDefinition

            expect(savedField.unique).toBe(true)
            expect(savedField.constraints).toBeDefined()

            // Verify nested structure
            const savedConstraints = savedField.constraints as {
                type: string
                constraints: Record<string, unknown>
            }
            expect(savedConstraints.type).toBe('string')
            expect(savedConstraints.constraints.pattern).toBe(EMAIL_REGEX)
            expect(savedConstraints.constraints.min_length).toBe(5)
            expect(savedConstraints.constraints.max_length).toBe(255)
        })
    })
})
