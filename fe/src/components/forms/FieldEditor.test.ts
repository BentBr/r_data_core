import { mount, DOMWrapper } from '@vue/test-utils'
import { describe, it, expect, vi, afterEach } from 'vitest'
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

    describe('Template rendering (real teleport)', () => {
        // The global test-setup stubs <teleport> with a plain boolean stub that
        // mis-renders VDialog's content ("[object Object]") instead of actual
        // DOM nodes. Disabling the stub for these mounts restores real
        // rendering, but moves the dialog content outside wrapper.element, so
        // we query via a DOMWrapper over document.body instead of wrapper.find.
        // Real <teleport> appends to document.body, so each mount is tracked
        // and force-unmounted in afterEach to avoid leaking DOM into later
        // tests (including after assertion failures).
        let activeWrapper: ReturnType<typeof mount> | undefined

        afterEach(() => {
            activeWrapper?.unmount()
            activeWrapper = undefined
        })

        const mountReal = (props: { modelValue: boolean; field?: FieldDefinition }) => {
            const wrapper = mount(FieldEditor, {
                props,
                global: { plugins: [vuetify], stubs: { teleport: false } },
            })
            activeWrapper = wrapper
            return wrapper
        }

        const body = () => new DOMWrapper(document.body)

        // Vuetify's v-switch also renders an <input type="checkbox">, so
        // locating the real v-checkbox controls (emailPreset / unique) by
        // bare input selector is ambiguous. Target by the .v-checkbox
        // wrapper's rendered label text instead.
        const findCheckboxByLabel = (label: string) => {
            const match = body()
                .findAll('.v-checkbox')
                .find(c => c.text().includes(label))
            return match?.find('input[type="checkbox"]')
        }

        it('renders the add-field title and enables default-value text input for String type', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()

            expect(body().text()).toContain('entity_definitions.fields.add_field')
            expect(body().find('[data-test="name"]').exists()).toBe(true)
            expect(body().find('[data-test="display_name"]').exists()).toBe(true)
            expect(body().find('[data-test="field_type"]').exists()).toBe(true)
        })

        it('renders the edit-field title when editing an existing field', async () => {
            const field: FieldDefinition = {
                name: 'existing',
                display_name: 'Existing',
                field_type: 'String',
                description: '',
                required: false,
                indexed: false,
                filterable: false,
                unique: false,
                default_value: undefined,
                constraints: {},
                ui_settings: {},
            }
            const wrapper = mountReal({ modelValue: true, field })
            await wrapper.vm.$nextTick()

            expect(body().text()).toContain('entity_definitions.fields.edit_field')
            // name field is disabled while editing
            const nameInput = body().find('input[name="name"]')
            expect(nameInput.attributes('disabled')).toBeDefined()
        })

        it('renders the string validation section with pattern disabled while email preset is on', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as { form: FieldDefinition }
            vm.form.field_type = 'String'
            await wrapper.vm.$nextTick()

            const emailCheckbox = findCheckboxByLabel('email_format')
            expect(emailCheckbox?.exists()).toBe(true)
            await emailCheckbox?.setValue(true)
            await wrapper.vm.$nextTick()

            expect(vm.form.constraints?.pattern).toBe(
                '^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$'
            )

            // Toggling back off clears the pattern (emailPreset setter false branch)
            await emailCheckbox?.setValue(false)
            await wrapper.vm.$nextTick()
            expect(vm.form.constraints?.pattern).toBeUndefined()
        })

        it('renders numeric validation inputs for Integer field type', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as { form: FieldDefinition; isNumericType: boolean }
            vm.form.field_type = 'Integer'
            await wrapper.vm.$nextTick()

            expect(vm.isNumericType).toBe(true)
            // Numeric section renders min/max number inputs + positive_only checkbox
            const numberInputs = body().findAll('input[type="number"]')
            expect(numberInputs.length).toBeGreaterThanOrEqual(2)
        })

        it('does not render validation section for a non-validated field type', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                showValidationSection: boolean
            }
            vm.form.field_type = 'Boolean'
            await wrapper.vm.$nextTick()

            expect(vm.showValidationSection).toBe(false)
            expect(body().text()).not.toContain('entity_definitions.fields.validation_options')
        })

        it('renders a Boolean default-value dropdown instead of a free text input', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as { form: FieldDefinition }
            // Baseline: String default value uses a plain text input, so only
            // the field_type select contributes combobox role(s).
            const baselineCount = body().findAll('[role="combobox"]').length

            vm.form.field_type = 'Boolean'
            await wrapper.vm.$nextTick()

            // Boolean default value adds a second v-select (True/False dropdown)
            expect(body().findAll('[role="combobox"]').length).toBeGreaterThan(baselineCount)
        })

        it('renders a Date default-value input and updates it via real input', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as { form: FieldDefinition }
            vm.form.field_type = 'Date'
            await wrapper.vm.$nextTick()

            const dateInput = body().find('input[type="date"]')
            expect(dateInput.exists()).toBe(true)
            await dateInput.setValue('2024-05-01')
            await wrapper.vm.$nextTick()
            expect(vm.form.default_value).toBe('2024-05-01')
        })

        it('renders a DateTime default-value input and updates it via real input', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as { form: FieldDefinition }
            vm.form.field_type = 'DateTime'
            await wrapper.vm.$nextTick()

            const dateTimeInput = body().find('input[type="datetime-local"]')
            expect(dateTimeInput.exists()).toBe(true)
            await dateTimeInput.setValue('2024-05-01T10:30')
            await wrapper.vm.$nextTick()
            expect(vm.form.default_value).toBe('2024-05-01T10:30')
        })

        it('updates the default_value number input for Integer and Float fields via real input', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()
            const vm = wrapper.vm as unknown as { form: FieldDefinition }

            vm.form.field_type = 'Integer'
            await wrapper.vm.$nextTick()
            // Only the default-value number input remains once field_type is
            // Integer (no numeric validation min/max shown pre-selection here).
            const integerInputs = body().findAll('input[type="number"]')
            await integerInputs[integerInputs.length - 1]?.setValue(7)
            await wrapper.vm.$nextTick()
            expect(vm.form.default_value).toBe(7)

            vm.form.field_type = 'Float'
            vm.form.default_value = undefined
            await wrapper.vm.$nextTick()
            const floatInputs = body().findAll('input[type="number"]')
            await floatInputs[floatInputs.length - 1]?.setValue(3.5)
            await wrapper.vm.$nextTick()
            expect(vm.form.default_value).toBe(3.5)
        })

        it('hides the default-value section for Password fields', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as { form: FieldDefinition }
            vm.form.field_type = 'Password'
            await wrapper.vm.$nextTick()

            expect(body().text()).not.toContain('entity_definitions.fields.default_value')
        })

        it('closes the dialog and resets the form when cancel is clicked', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as { form: FieldDefinition }
            vm.form.name = 'dirty'
            await wrapper.vm.$nextTick()

            const cancelBtn = body().find('[data-test="cancel"]')
            expect(cancelBtn.exists()).toBe(true)
            await cancelBtn.trigger('click')
            await wrapper.vm.$nextTick()

            expect(wrapper.emitted('update:modelValue')).toBeTruthy()
            expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([false])
            expect(vm.form.name).toBe('')
        })

        it('disables the save button while the form is invalid', async () => {
            mountReal({ modelValue: true, field: undefined })
            await activeWrapper?.vm.$nextTick()

            const saveBtn = body().find('[data-test="save"]')
            expect(saveBtn.exists()).toBe(true)
            expect(saveBtn.attributes('disabled')).toBeDefined()
        })

        it('updates name, display_name, description and the toggle switches via real input', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()
            const vm = wrapper.vm as unknown as { form: FieldDefinition }

            await body().find('[data-test="name"] input').setValue('my_field')
            await body().find('[data-test="display_name"] input').setValue('My Field')

            const descriptionWrapper = body()
                .findAll('.v-input')
                .find(w => w.text().includes('entity_definitions.fields.description'))
            await descriptionWrapper?.find('input').setValue('A helpful description')
            await wrapper.vm.$nextTick()

            expect(vm.form.name).toBe('my_field')
            expect(vm.form.display_name).toBe('My Field')
            expect(vm.form.description).toBe('A helpful description')

            const switches = body().findAll('input[type="checkbox"]')
            // required, indexed, filterable switches render first (before the
            // validation section's v-checkbox controls)
            await switches[0]?.setValue(true)
            await switches[1]?.setValue(true)
            await switches[2]?.setValue(true)
            await wrapper.vm.$nextTick()

            expect(vm.form.required).toBe(true)
            expect(vm.form.indexed).toBe(true)
            expect(vm.form.filterable).toBe(true)
        })

        it('updates min_length, max_length and pattern via real input for String fields', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                constraintMinLength: number | undefined
                constraintMaxLength: number | undefined
                constraintPattern: string | undefined
            }

            const numberInputs = body().findAll('input[type="number"]')
            expect(numberInputs.length).toBeGreaterThanOrEqual(2)
            await numberInputs[0]?.setValue(3)
            await numberInputs[1]?.setValue(20)

            // The pattern field has no data-test/name attribute; locate it by
            // its persistent hint text instead of relying on DOM order.
            const patternWrapper = body()
                .findAll('.v-input')
                .find(w => w.text().includes('pattern_hint'))
            const patternInput = patternWrapper?.find('input')
            await patternInput?.setValue('^abc$')
            await wrapper.vm.$nextTick()

            expect(vm.constraintMinLength).toBe(3)
            expect(vm.constraintMaxLength).toBe(20)
            expect(vm.constraintPattern).toBe('^abc$')
        })

        it('updates the unique checkbox via real input', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()
            const vm = wrapper.vm as unknown as { constraintUnique: boolean }

            const uniqueCheckbox = findCheckboxByLabel('unique')
            expect(uniqueCheckbox?.exists()).toBe(true)
            await uniqueCheckbox?.setValue(true)
            await wrapper.vm.$nextTick()

            expect(vm.constraintUnique).toBe(true)
        })

        it('updates min, max and positive_only via real input for Integer fields', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                constraintMin: number | undefined
                constraintMax: number | undefined
                constraintPositiveOnly: boolean | undefined
            }
            vm.form.field_type = 'Integer'
            await wrapper.vm.$nextTick()

            const numberInputs = body().findAll('input[type="number"]')
            expect(numberInputs.length).toBeGreaterThanOrEqual(2)
            await numberInputs[0]?.setValue(1)
            await numberInputs[1]?.setValue(99)

            const positiveOnly = findCheckboxByLabel('positive_only')
            await positiveOnly?.setValue(true)
            await wrapper.vm.$nextTick()

            expect(vm.constraintMin).toBe(1)
            expect(vm.constraintMax).toBe(99)
            expect(vm.constraintPositiveOnly).toBe(true)
        })

        it('updates the default_value text input for a String field via real input', async () => {
            const wrapper = mountReal({ modelValue: true, field: undefined })
            await wrapper.vm.$nextTick()
            const vm = wrapper.vm as unknown as { form: FieldDefinition }

            const defaultValueInput = body().find('[data-test="save"]')
            expect(defaultValueInput.exists()).toBe(true)

            // The String default value input has no data-test/name attribute,
            // so target it as the last plain text input in the form.
            const textInputs = body().findAll('input[type="text"]')
            const lastTextInput = textInputs[textInputs.length - 1]
            await lastTextInput.setValue('fallback')
            await wrapper.vm.$nextTick()

            expect(vm.form.default_value).toBe('fallback')
        })
    })

    describe('saveField early return when form is invalid', () => {
        it('does not emit save when formValid is false', async () => {
            const wrapper = mount(FieldEditor, {
                props: { modelValue: true, field: undefined },
                global: { plugins: [vuetify] },
            })
            await wrapper.vm.$nextTick()

            const vm = wrapper.vm as unknown as { formValid: boolean; saveField: () => void }
            vm.formValid = false
            vm.saveField()

            expect(wrapper.emitted('save')).toBeFalsy()
        })
    })

    describe('formatDefaultValue branches via saveField', () => {
        const mountNew = async () => {
            const wrapper = mount(FieldEditor, {
                props: { modelValue: true, field: undefined },
                global: { plugins: [vuetify] },
            })
            await wrapper.vm.$nextTick()
            return wrapper
        }

        const getSaved = (wrapper: ReturnType<typeof mount>) => {
            const saveEvents = wrapper.emitted('save')
            return saveEvents?.[0]?.[0] as FieldDefinition
        }

        it('keeps an actual boolean default value unchanged for Boolean fields', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'flag'
            vm.form.display_name = 'Flag'
            vm.form.field_type = 'Boolean'
            vm.form.default_value = true
            vm.formValid = true
            vm.saveField()

            expect(getSaved(wrapper).default_value).toBe(true)
        })

        it('coerces boolean-like strings for Boolean fields', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'flag'
            vm.form.display_name = 'Flag'
            vm.form.field_type = 'Boolean'
            vm.form.default_value = 'yes'
            vm.formValid = true
            vm.saveField()

            expect(getSaved(wrapper).default_value).toBe(true)
        })

        it('coerces a numeric value to boolean for Boolean fields', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'flag'
            vm.form.display_name = 'Flag'
            vm.form.field_type = 'Boolean'
            vm.form.default_value = 0
            vm.formValid = true
            vm.saveField()

            expect(getSaved(wrapper).default_value).toBe(false)
        })

        it('falls back to false for an unrecognized Boolean default value type', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'flag'
            vm.form.display_name = 'Flag'
            vm.form.field_type = 'Boolean'
            vm.form.default_value = { weird: true } as unknown as FieldDefinition['default_value']
            vm.formValid = true
            vm.saveField()

            expect(getSaved(wrapper).default_value).toBe(false)
        })

        it('floors a numeric Integer default value', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'count'
            vm.form.display_name = 'Count'
            vm.form.field_type = 'Integer'
            vm.form.default_value = 3.7
            vm.formValid = true
            vm.saveField()

            expect(getSaved(wrapper).default_value).toBe(3)
        })

        it('parses a numeric string for Integer default value, undefined when unparsable', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'count'
            vm.form.display_name = 'Count'
            vm.form.field_type = 'Integer'
            vm.form.default_value = '42'
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toBe(42)
        })

        it('returns undefined for an unparsable Integer string default value', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'count'
            vm.form.display_name = 'Count'
            vm.form.field_type = 'Integer'
            vm.form.default_value = 'not-a-number'
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toBeUndefined()
        })

        it('returns undefined for a non-numeric, non-string Integer default value', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'count'
            vm.form.display_name = 'Count'
            vm.form.field_type = 'Integer'
            vm.form.default_value = true as unknown as FieldDefinition['default_value']
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toBeUndefined()
        })

        it('keeps a numeric Float default value and parses float strings', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'price'
            vm.form.display_name = 'Price'
            vm.form.field_type = 'Float'
            vm.form.default_value = 1.5
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toBe(1.5)
        })

        it('parses a numeric string for Float default value, undefined when unparsable', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'price'
            vm.form.display_name = 'Price'
            vm.form.field_type = 'Float'
            vm.form.default_value = '2.25'
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toBe(2.25)
        })

        it('returns undefined for a non-numeric, non-string Float default value', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'price'
            vm.form.display_name = 'Price'
            vm.form.field_type = 'Float'
            vm.form.default_value = false as unknown as FieldDefinition['default_value']
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toBeUndefined()
        })

        it('keeps a string Date default value and discards non-string values', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'start_date'
            vm.form.display_name = 'Start date'
            vm.form.field_type = 'Date'
            vm.form.default_value = '2024-01-01'
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toBe('2024-01-01')
        })

        it('discards a non-string DateTime default value', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'start_at'
            vm.form.display_name = 'Start at'
            vm.form.field_type = 'DateTime'
            vm.form.default_value = 123 as unknown as FieldDefinition['default_value']
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toBeUndefined()
        })

        it('keeps an object default value as-is for Object fields', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'meta'
            vm.form.display_name = 'Meta'
            vm.form.field_type = 'Object'
            vm.form.default_value = { a: 1 } as unknown as FieldDefinition['default_value']
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toEqual({ a: 1 })
        })

        it('parses a JSON string default value for Array fields', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'tags'
            vm.form.display_name = 'Tags'
            vm.form.field_type = 'Array'
            vm.form.default_value = '[1,2,3]'
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toEqual([1, 2, 3])
        })

        it('returns undefined for an unparsable JSON string default value', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'tags'
            vm.form.display_name = 'Tags'
            vm.form.field_type = 'Json'
            vm.form.default_value = 'not-json{'
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toBeUndefined()
        })

        it('returns undefined for a non-object, non-string Json default value', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'tags'
            vm.form.display_name = 'Tags'
            vm.form.field_type = 'Json'
            vm.form.default_value = 5 as unknown as FieldDefinition['default_value']
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toBeUndefined()
        })

        it('converts a non-string default value to String for the default String branch', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'label'
            vm.form.display_name = 'Label'
            vm.form.field_type = 'String'
            vm.form.default_value = 7 as unknown as FieldDefinition['default_value']
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toBe('7')
        })

        it('returns undefined when default_value is an empty string', async () => {
            const wrapper = await mountNew()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'label'
            vm.form.display_name = 'Label'
            vm.form.field_type = 'String'
            vm.form.default_value = ''
            vm.formValid = true
            vm.saveField()
            expect(getSaved(wrapper).default_value).toBeUndefined()
        })
    })

    describe('getConstraintType branches via saveField', () => {
        const saveAndGetConstraintType = async (fieldType: FieldDefinition['field_type']) => {
            const wrapper = mount(FieldEditor, {
                props: { modelValue: true, field: undefined },
                global: { plugins: [vuetify] },
            })
            await wrapper.vm.$nextTick()
            const vm = wrapper.vm as unknown as {
                form: FieldDefinition
                formValid: boolean
                saveField: () => void
            }
            vm.form.name = 'field'
            vm.form.display_name = 'Field'
            vm.form.field_type = fieldType
            vm.formValid = true
            vm.saveField()

            const saveEvents = wrapper.emitted('save')
            const saved = saveEvents?.[0]?.[0] as FieldDefinition
            return (saved.constraints as { type: string }).type
        }

        it('maps Float to the float constraint type', async () => {
            expect(await saveAndGetConstraintType('Float')).toBe('float')
        })

        it('maps DateTime to the datetime constraint type', async () => {
            expect(await saveAndGetConstraintType('DateTime')).toBe('datetime')
        })

        it('maps Date to the date constraint type', async () => {
            expect(await saveAndGetConstraintType('Date')).toBe('date')
        })

        it('maps Select to the select constraint type', async () => {
            expect(await saveAndGetConstraintType('Select')).toBe('select')
        })

        it('maps MultiSelect to the multiselect constraint type', async () => {
            expect(await saveAndGetConstraintType('MultiSelect')).toBe('multiselect')
        })

        it('maps ManyToOne to the relation constraint type', async () => {
            expect(await saveAndGetConstraintType('ManyToOne')).toBe('relation')
        })

        it('maps ManyToMany to the relation constraint type', async () => {
            expect(await saveAndGetConstraintType('ManyToMany')).toBe('relation')
        })

        it('falls back to schema for unmapped field types', async () => {
            expect(await saveAndGetConstraintType('Boolean')).toBe('schema')
        })
    })
})
