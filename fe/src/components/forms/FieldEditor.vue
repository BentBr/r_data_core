<template>
    <v-dialog
        v-model="showDialog"
        max-width="800px"
        persistent
    >
        <v-card>
            <v-card-title class="text-h5 pa-4">
                {{ isEditing ? 'Edit Field' : 'Add Field' }}
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
                                label="Field Name"
                                :rules="[
                                    v => !!v || 'Field name is required',
                                    v =>
                                        /^[a-zA-Z_][a-zA-Z0-9_]*$/.test(v) ||
                                        'Field name must be alphanumeric with underscores only',
                                ]"
                                required
                                :disabled="isEditing"
                            />
                        </v-col>
                        <v-col cols="6">
                            <v-text-field
                                v-model="form.display_name"
                                label="Display Name"
                                :rules="[v => !!v || 'Display name is required']"
                                required
                            />
                        </v-col>
                    </v-row>

                    <v-row>
                        <v-col cols="6">
                            <v-select
                                v-model="form.field_type"
                                :items="fieldTypes"
                                label="Field Type"
                                :rules="[v => !!v || 'Field type is required']"
                                required
                            />
                        </v-col>
                        <v-col cols="6">
                            <v-text-field
                                v-model="form.description"
                                label="Description (Optional)"
                            />
                        </v-col>
                    </v-row>

                    <v-row>
                        <v-col cols="4">
                            <v-switch
                                v-model="form.required"
                                label="Required"
                            />
                        </v-col>
                        <v-col cols="4">
                            <v-switch
                                v-model="form.indexed"
                                label="Indexed"
                            />
                        </v-col>
                        <v-col cols="4">
                            <v-switch
                                v-model="form.filterable"
                                label="Filterable"
                            />
                        </v-col>
                    </v-row>

                    <v-row v-if="showDefaultValue">
                        <v-col cols="12">
                            <v-text-field
                                v-model="form.default_value"
                                label="Default Value (Optional)"
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
                    Cancel
                </v-btn>
                <v-btn
                    color="primary"
                    :disabled="!formValid"
                    @click="saveField"
                >
                    {{ isEditing ? 'Update' : 'Add' }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { ref, computed, watch } from 'vue'
    import type { FieldDefinition } from '@/types/schemas'

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

    const closeDialog = () => {
        showDialog.value = false
        resetForm()
    }

    const saveField = () => {
        if (!formValid.value) {
            return
        }

        // Ensure constraints and ui_settings are always objects, not null
        const fieldToSave = {
            ...form.value,
            constraints: form.value.constraints || {},
            ui_settings: form.value.ui_settings || {},
        }

        emit('save', fieldToSave)
        closeDialog()
    }

    watch(
        () => props.field,
        newField => {
            if (newField) {
                form.value = {
                    ...newField,
                    constraints: newField.constraints || {},
                    ui_settings: newField.ui_settings || {},
                }
            } else {
                resetForm()
            }
        },
        { immediate: true }
    )
</script>
