<template>
    <v-dialog
        v-model="showDialog"
        :max-width="getDialogMaxWidth('form')"
        persistent
        :retain-focus="false"
    >
        <v-card>
            <v-card-title class="text-h5 pa-6">
                {{
                    isEditing
                        ? t('entity_definitions.fields.edit_field')
                        : t('entity_definitions.fields.add_field')
                }}
            </v-card-title>
            <v-card-text class="pa-6">
                <v-form
                    ref="formRef"
                    v-model="formValid"
                >
                    <v-row>
                        <v-col cols="6">
                            <v-text-field
                                v-model="form.name"
                                data-test="name"
                                name="name"
                                :label="t('entity_definitions.fields.field_name')"
                                :rules="[
                                    v => !!v ?? t('entity_definitions.fields.field_name_required'),
                                    v =>
                                        /^[a-zA-Z_][a-zA-Z0-9_]*$/.test(v) ||
                                        t('entity_definitions.fields.field_name_invalid'),
                                ]"
                                required
                                :disabled="isEditing"
                            />
                        </v-col>
                        <v-col cols="6">
                            <v-text-field
                                v-model="form.display_name"
                                data-test="display_name"
                                name="display_name"
                                :label="t('entity_definitions.fields.display_name')"
                                :rules="[
                                    v =>
                                        !!v ?? t('entity_definitions.fields.display_name_required'),
                                ]"
                                required
                            />
                        </v-col>
                    </v-row>

                    <v-row>
                        <v-col cols="6">
                            <v-select
                                v-model="form.field_type"
                                data-test="field_type"
                                name="field_type"
                                :items="fieldTypes"
                                item-title="title"
                                item-value="value"
                                :label="t('entity_definitions.fields.field_type')"
                                :rules="[
                                    v => !!v ?? t('entity_definitions.fields.field_type_required'),
                                ]"
                                required
                            />
                        </v-col>
                        <v-col cols="6">
                            <v-text-field
                                v-model="form.description"
                                :label="t('entity_definitions.fields.description') + ' (Optional)'"
                            />
                        </v-col>
                    </v-row>

                    <v-row>
                        <v-col cols="4">
                            <v-switch
                                v-model="form.required"
                                :label="t('entity_definitions.fields.required')"
                            />
                        </v-col>
                        <v-col cols="4">
                            <v-switch
                                v-model="form.indexed"
                                :label="t('entity_definitions.fields.indexed')"
                            />
                        </v-col>
                        <v-col cols="4">
                            <v-switch
                                v-model="form.filterable"
                                :label="t('entity_definitions.fields.filterable')"
                            />
                        </v-col>
                    </v-row>

                    <!-- Validation Options Section -->
                    <v-row v-if="showValidationSection">
                        <v-col cols="12">
                            <v-divider class="my-2" />
                            <div class="text-subtitle-2 mb-2">
                                {{ t('entity_definitions.fields.validation_options') }}
                            </div>
                        </v-col>
                    </v-row>

                    <!-- String validation (String, Text, Wysiwyg) -->
                    <template v-if="isStringType">
                        <v-row>
                            <v-col cols="4">
                                <v-text-field
                                    v-model.number="constraintMinLength"
                                    :label="t('entity_definitions.fields.min_length')"
                                    type="number"
                                    min="0"
                                />
                            </v-col>
                            <v-col cols="4">
                                <v-text-field
                                    v-model.number="constraintMaxLength"
                                    :label="t('entity_definitions.fields.max_length')"
                                    type="number"
                                    min="0"
                                />
                            </v-col>
                            <v-col cols="4">
                                <v-checkbox
                                    v-model="emailPreset"
                                    :label="t('entity_definitions.fields.email_format')"
                                    density="compact"
                                />
                            </v-col>
                        </v-row>
                        <v-row>
                            <v-col cols="12">
                                <v-text-field
                                    v-model="constraintPattern"
                                    :label="t('entity_definitions.fields.pattern')"
                                    :hint="t('entity_definitions.fields.pattern_hint')"
                                    :disabled="emailPreset"
                                    persistent-hint
                                />
                            </v-col>
                        </v-row>
                    </template>

                    <!-- Numeric validation (Integer, Float) -->
                    <template v-if="isNumericType">
                        <v-row>
                            <v-col cols="4">
                                <v-text-field
                                    v-model.number="constraintMin"
                                    :label="t('entity_definitions.fields.min_value')"
                                    type="number"
                                />
                            </v-col>
                            <v-col cols="4">
                                <v-text-field
                                    v-model.number="constraintMax"
                                    :label="t('entity_definitions.fields.max_value')"
                                    type="number"
                                />
                            </v-col>
                            <v-col cols="4">
                                <v-checkbox
                                    v-model="constraintPositiveOnly"
                                    :label="t('entity_definitions.fields.positive_only')"
                                    density="compact"
                                />
                            </v-col>
                        </v-row>
                    </template>

                    <!-- Uniqueness (String, Text, Integer, Uuid) -->
                    <v-row v-if="supportsUniqueness">
                        <v-col cols="12">
                            <v-checkbox
                                v-model="constraintUnique"
                                :label="t('entity_definitions.fields.unique')"
                                :hint="t('entity_definitions.fields.unique_hint')"
                                persistent-hint
                                density="compact"
                            />
                        </v-col>
                    </v-row>

                    <v-row v-if="showDefaultValue">
                        <v-col cols="12">
                            <!-- Boolean: Dropdown -->
                            <v-select
                                v-if="form.field_type === 'Boolean'"
                                v-model="form.default_value"
                                :label="
                                    t('entity_definitions.fields.default_value') + ' (Optional)'
                                "
                                :items="[
                                    { title: 'True', value: true },
                                    { title: 'False', value: false },
                                ]"
                                clearable
                            />
                            <!-- Integer: Number input -->
                            <v-text-field
                                v-else-if="form.field_type === 'Integer'"
                                v-model.number="form.default_value"
                                :label="
                                    t('entity_definitions.fields.default_value') + ' (Optional)'
                                "
                                type="number"
                            />
                            <!-- Float: Number input -->
                            <v-text-field
                                v-else-if="form.field_type === 'Float'"
                                v-model.number="form.default_value"
                                :label="
                                    t('entity_definitions.fields.default_value') + ' (Optional)'
                                "
                                type="number"
                                step="any"
                            />
                            <!-- Date: Date picker -->
                            <v-text-field
                                v-else-if="form.field_type === 'Date'"
                                v-model="form.default_value"
                                :label="
                                    t('entity_definitions.fields.default_value') + ' (Optional)'
                                "
                                type="date"
                            />
                            <!-- DateTime: DateTime picker -->
                            <v-text-field
                                v-else-if="form.field_type === 'DateTime'"
                                v-model="form.default_value"
                                :label="
                                    t('entity_definitions.fields.default_value') + ' (Optional)'
                                "
                                type="datetime-local"
                            />
                            <!-- String/Text: Text input -->
                            <v-text-field
                                v-else
                                v-model="form.default_value"
                                :label="
                                    t('entity_definitions.fields.default_value') + ' (Optional)'
                                "
                            />
                        </v-col>
                    </v-row>
                </v-form>
            </v-card-text>
            <v-card-actions class="pa-4 px-6">
                <v-spacer />
                <v-btn
                    data-test="cancel"
                    variant="text"
                    color="mutedForeground"
                    @click="closeDialog"
                >
                    {{ t('common.cancel') }}
                </v-btn>
                <v-btn
                    data-test="save"
                    color="primary"
                    variant="flat"
                    :disabled="!formValid"
                    @click="saveField"
                >
                    {{
                        isEditing
                            ? t('entity_definitions.fields.update')
                            : t('entity_definitions.fields.add')
                    }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { ref, computed, watch } from 'vue'
    import type { FieldDefinition } from '@/types/schemas'
    import { useTranslations } from '@/composables/useTranslations'
    import { getDialogMaxWidth } from '@/design-system/components'

    interface Props {
        modelValue: boolean
        field?: FieldDefinition
    }

    interface Emits {
        (e: 'update:modelValue', value: boolean): void

        (e: 'save', field: FieldDefinition): void
    }

    const props = defineProps<Props>()
    const emit = defineEmits<Emits>()
    const { t } = useTranslations()

    const showDialog = computed({
        get: () => props.modelValue,
        set: value => emit('update:modelValue', value),
    })

    const isEditing = computed(() => !!props.field)

    const formValid = ref(false)
    const formRef = ref<HTMLFormElement | null>(null)

    const fieldTypes = [
        { title: 'String', value: 'String' },
        { title: 'Text', value: 'Text' },
        { title: 'Wysiwyg', value: 'Wysiwyg' },
        { title: 'Integer', value: 'Integer' },
        { title: 'Float', value: 'Float' },
        { title: 'Boolean', value: 'Boolean' },
        { title: 'Date', value: 'Date' },
        { title: 'DateTime', value: 'DateTime' },
        { title: 'Json (any)', value: 'Json' },
        { title: 'Json Object', value: 'Object' },
        { title: 'Json Array', value: 'Array' },
        { title: 'Uuid', value: 'Uuid' },
        { title: 'ManyToOne', value: 'ManyToOne' },
        { title: 'ManyToMany', value: 'ManyToMany' },
        { title: 'Select', value: 'Select' },
        { title: 'MultiSelect', value: 'MultiSelect' },
        { title: 'Image', value: 'Image' },
        { title: 'File', value: 'File' },
        { title: 'Password', value: 'Password' },
    ]

    const form = ref<FieldDefinition>({
        name: '',
        display_name: '',
        field_type: 'String',
        description: '',
        required: false,
        indexed: false,
        filterable: false,
        unique: false,
        default_value: undefined,
        constraints: {},
        ui_settings: {},
    })

    const showDefaultValue = computed(() => {
        // Password fields should not have default values
        if (form.value.field_type === 'Password') {
            return false
        }
        return ['String', 'Text', 'Integer', 'Float', 'Boolean', 'Date', 'DateTime'].includes(
            form.value.field_type
        )
    })

    // Validation section computed properties
    const EMAIL_REGEX = '^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$'

    const isStringType = computed(() =>
        ['String', 'Text', 'Wysiwyg', 'Password'].includes(form.value.field_type)
    )
    const isNumericType = computed(() => ['Integer', 'Float'].includes(form.value.field_type))
    const supportsUniqueness = computed(() =>
        ['String', 'Text', 'Integer', 'Uuid'].includes(form.value.field_type)
    )
    const showValidationSection = computed(
        () => isStringType.value || isNumericType.value || supportsUniqueness.value
    )

    const emailPreset = computed({
        get: () => form.value.constraints?.pattern === EMAIL_REGEX,
        set: (value: boolean) => {
            form.value.constraints ??= {}
            form.value.constraints.pattern = value ? EMAIL_REGEX : undefined
        },
    })

    // Constraint computed properties with safe getters/setters
    const ensureConstraints = () => {
        form.value.constraints ??= {}
    }

    const constraintMinLength = computed({
        get: () => form.value.constraints?.min_length as number | undefined,
        set: (value: number | undefined) => {
            ensureConstraints()
            form.value.constraints!.min_length = value
        },
    })

    const constraintMaxLength = computed({
        get: () => form.value.constraints?.max_length as number | undefined,
        set: (value: number | undefined) => {
            ensureConstraints()
            form.value.constraints!.max_length = value
        },
    })

    const constraintPattern = computed({
        get: () => form.value.constraints?.pattern as string | undefined,
        set: (value: string | undefined) => {
            ensureConstraints()
            form.value.constraints!.pattern = value
        },
    })

    const constraintMin = computed({
        get: () => form.value.constraints?.min as number | undefined,
        set: (value: number | undefined) => {
            ensureConstraints()
            form.value.constraints!.min = value
        },
    })

    const constraintMax = computed({
        get: () => form.value.constraints?.max as number | undefined,
        set: (value: number | undefined) => {
            ensureConstraints()
            form.value.constraints!.max = value
        },
    })

    const constraintPositiveOnly = computed({
        get: () => form.value.constraints?.positive_only as boolean | undefined,
        set: (value: boolean | undefined) => {
            ensureConstraints()
            form.value.constraints!.positive_only = value
        },
    })

    // Unique is at root level (DB-level constraint), not in constraints
    const constraintUnique = computed({
        get: () => form.value.unique ?? false,
        set: (value: boolean) => {
            form.value.unique = value
        },
    })

    const resetForm = () => {
        form.value = {
            name: '',
            display_name: '',
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
    }

    // Watch for field changes to populate form when editing
    watch(
        () => props.field,
        newField => {
            if (newField) {
                // API returns nested constraints: { type: "string", constraints: { pattern: "..." } }
                // Extract inner constraints for the flat form structure
                const apiConstraints = newField.constraints as
                    | { type?: string; constraints?: Record<string, unknown> }
                    | undefined
                const innerConstraints = apiConstraints?.constraints ?? {}

                // Editing existing field - populate form with field data
                form.value = {
                    name: newField.name,
                    display_name: newField.display_name,
                    field_type: newField.field_type,
                    description: newField.description ?? '',
                    required: newField.required,
                    indexed: newField.indexed,
                    filterable: newField.filterable,
                    unique: newField.unique ?? false,
                    default_value: newField.default_value,
                    // Use flat structure internally (extracted from nested API structure)
                    constraints: innerConstraints,
                    ui_settings: newField.ui_settings ?? {},
                }
            } else {
                // Adding new field - reset form
                resetForm()
            }
        },
        { immediate: true }
    )

    const closeDialog = () => {
        showDialog.value = false
        resetForm()
    }

    // Format default value to proper type
    const formatDefaultValue = (value: unknown, fieldType: string): unknown => {
        if (value === null || value === undefined || value === '') {
            return undefined
        }

        switch (fieldType) {
            case 'Boolean':
                if (typeof value === 'boolean') {
                    return value
                }
                if (typeof value === 'string') {
                    const lower = value.toLowerCase()
                    return lower === 'true' || lower === '1' || lower === 'yes' || lower === 'on'
                }
                if (typeof value === 'number') {
                    return value !== 0
                }
                return false
            case 'Integer':
                if (typeof value === 'number') {
                    return Math.floor(value)
                }
                if (typeof value === 'string') {
                    const parsed = parseInt(value, 10)
                    return isNaN(parsed) ? undefined : parsed
                }
                return undefined
            case 'Float':
                if (typeof value === 'number') {
                    return value
                }
                if (typeof value === 'string') {
                    const parsed = parseFloat(value)
                    return isNaN(parsed) ? undefined : parsed
                }
                return undefined
            case 'Date':
            case 'DateTime':
                // Keep as string for date/datetime
                return typeof value === 'string' ? value : undefined
            case 'Object':
            case 'Array':
            case 'Json':
                // If already an object/array, return as-is
                if (typeof value === 'object') {
                    return value
                }
                // Try to parse JSON string
                if (typeof value === 'string') {
                    try {
                        return JSON.parse(value)
                    } catch {
                        return undefined
                    }
                }
                return undefined
            default:
                // String, Text, etc. - keep as string
                return typeof value === 'string' ? value : String(value)
        }
    }

    // Get the constraint type based on field type
    const getConstraintType = (fieldType: string): string => {
        switch (fieldType) {
            case 'String':
            case 'Text':
            case 'Wysiwyg':
            case 'Password':
                return 'string'
            case 'Integer':
                return 'integer'
            case 'Float':
                return 'float'
            case 'DateTime':
                return 'datetime'
            case 'Date':
                return 'date'
            case 'Select':
                return 'select'
            case 'MultiSelect':
                return 'multiselect'
            case 'ManyToOne':
            case 'ManyToMany':
                return 'relation'
            default:
                return 'schema'
        }
    }

    const saveField = () => {
        if (!formValid.value) {
            return
        }

        // Format default value to proper type
        const formattedDefaultValue =
            form.value.default_value !== undefined
                ? formatDefaultValue(form.value.default_value, form.value.field_type)
                : undefined

        // Format constraints back to API nested structure: { type: "string", constraints: { ... } }
        const constraintType = getConstraintType(form.value.field_type)
        const formattedConstraints = {
            type: constraintType,
            constraints: form.value.constraints ?? {},
        }

        // Ensure constraints and ui_settings are always objects, not null
        // unique is at root level (DB-level constraint)
        const sanitizedField = {
            ...form.value,
            unique: form.value.unique ?? false,
            default_value: formattedDefaultValue,
            constraints: formattedConstraints,
            ui_settings: form.value.ui_settings ?? {},
        }

        emit('save', sanitizedField)
        closeDialog()
    }

    // Expose for tests
    defineExpose({
        form,
        formValid,
        formRef,
        showDefaultValue,
        saveField,
        isStringType,
        isNumericType,
        supportsUniqueness,
        showValidationSection,
        emailPreset,
        constraintMinLength,
        constraintMaxLength,
        constraintPattern,
        constraintMin,
        constraintMax,
        constraintPositiveOnly,
        constraintUnique,
    })
</script>
