<template>
    <v-dialog
        v-model="showDialog"
        max-width="800px"
        persistent
    >
        <v-card>
            <v-card-title class="text-h5 pa-4">
                {{ isEditing ? t('entity_definitions.fields.edit_field') : t('entity_definitions.fields.add_field') }}
            </v-card-title>
            <v-card-text>
                <v-form
                    ref="formRef"
                    v-model="formValid"
                >
                    <v-row>
                        <v-col cols="6">
                            <v-text-field
                                v-model="form.name"
                                :label="t('entity_definitions.fields.field_name')"
                                :rules="[
                                    v => !!v || t('entity_definitions.fields.field_name_required'),
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
                                :label="t('entity_definitions.fields.display_name')"
                                :rules="[v => !!v || t('entity_definitions.fields.display_name_required')]"
                                required
                            />
                        </v-col>
                    </v-row>

                    <v-row>
                        <v-col cols="6">
                            <v-select
                                v-model="form.field_type"
                                :items="fieldTypes"
                                :label="t('entity_definitions.fields.field_type')"
                                :rules="[v => !!v || t('entity_definitions.fields.field_type_required')]"
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

                    <v-row v-if="showDefaultValue">
                        <v-col cols="12">
                            <v-text-field
                                v-model="form.default_value"
                                :label="t('entity_definitions.fields.default_value') + ' (Optional)'"
                            />
                        </v-col>
                    </v-row>
                </v-form>
            </v-card-text>
            <v-card-actions class="pa-4">
                <v-spacer />
                <v-btn
                    color="grey"
                    variant="text"
                    @click="closeDialog"
                >
                    {{ t('common.cancel') }}
                </v-btn>
                <v-btn
                    color="primary"
                    :disabled="!formValid"
                    @click="saveField"
                >
                    {{ isEditing ? t('entity_definitions.fields.update') : t('entity_definitions.fields.add') }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { ref, computed, watch } from 'vue'
    import type { FieldDefinition } from '@/types/schemas'
    import { useTranslations } from '@/composables/useTranslations'

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
        'String',
        'Text',
        'Wysiwyg',
        'Integer',
        'Float',
        'Boolean',
        'Date',
        'DateTime',
        'Object',
        'Array',
        'UUID',
        'ManyToOne',
        'ManyToMany',
        'Select',
        'MultiSelect',
        'Image',
        'File',
    ]

    const form = ref<FieldDefinition>({
        name: '',
        display_name: '',
        field_type: 'String',
        description: '',
        required: false,
        indexed: false,
        filterable: false,
        default_value: undefined,
        constraints: {},
        ui_settings: {},
    })

    const showDefaultValue = computed(() => {
        return ['String', 'Text', 'Integer', 'Float', 'Boolean'].includes(form.value.field_type)
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
            default_value: undefined,
            constraints: {},
            ui_settings: {},
        }
    }

    // Watch for field changes to populate form when editing
    watch(() => props.field, (newField) => {
        if (newField) {
            // Editing existing field - populate form with field data
            form.value = {
                name: newField.name,
                display_name: newField.display_name,
                field_type: newField.field_type,
                description: newField.description || '',
                required: newField.required,
                indexed: newField.indexed,
                filterable: newField.filterable,
                default_value: newField.default_value,
                constraints: newField.constraints || {},
                ui_settings: newField.ui_settings || {},
            }
        } else {
            // Adding new field - reset form
            resetForm()
        }
    }, { immediate: true })

    const closeDialog = () => {
        showDialog.value = false
        resetForm()
    }

    const saveField = () => {
        if (!formValid.value) {
            return
        }

        // Ensure constraints and ui_settings are always objects, not null
        const sanitizedField = {
            ...form.value,
            constraints: form.value.constraints || {},
            ui_settings: form.value.ui_settings || {},
        }

        emit('save', sanitizedField)
        closeDialog()
    }
</script>
