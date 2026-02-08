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
})
