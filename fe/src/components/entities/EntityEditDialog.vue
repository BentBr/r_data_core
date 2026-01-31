<template>
    <v-dialog
        v-model="dialogVisible"
        :max-width="getDialogMaxWidth('form')"
        persistent
    >
        <v-card>
            <v-card-title class="d-flex align-center pa-6">
                <SmartIcon
                    icon="pencil"
                    size="md"
                    class="mr-3"
                />
                {{ t('entities.edit.title') }}
            </v-card-title>

            <v-card-text class="pa-6">
                <v-form
                    ref="form"
                    v-model="isValid"
                >
                    <!-- Entity Type Display (Read-only) -->
                    <v-text-field
                        :model-value="entity?.entity_type"
                        :label="t('entities.create.entity_type_label')"
                        readonly
                        variant="outlined"
                    />

                    <!-- Parent Entity Selection -->
                    <v-select
                        :model-value="parentUuidValue"
                        :items="availableParents"
                        item-title="title"
                        item-value="uuid"
                        :label="t('entities.create.parent_label')"
                        clearable
                        class="mt-4"
                        @update:model-value="(v: string | null) => (parentUuidValue = v)"
                    />

                    <!-- Published Switch -->
                    <v-switch
                        v-model="formData.data.published"
                        :label="t('entities.create.published_label')"
                        :hint="t('entities.create.published_hint')"
                        color="success"
                        inset
                        persistent-hint
                        class="mt-4"
                    />

                    <!-- Dynamic Fields based on Entity Definition -->
                    <div
                        v-if="entityDefinition"
                        class="mt-4"
                    >
                        <h4 class="text-subtitle-1 mb-3">{{ t('entities.create.data_label') }}</h4>

                        <v-row>
                            <v-col
                                v-for="field in entityDefinition.fields"
                                :key="field.name"
                                :cols="field.ui_settings?.width === 'full' ? 12 : 6"
                            >
                                <component
                                    :is="getFieldComponent(field.field_type)"
                                    v-model="formData.data[field.name]"
                                    :label="field.display_name"
                                    :hint="field.description"
                                    :rules="getFieldRules(field)"
                                    :error-messages="getFieldErrorMessages(field.name)"
                                    :required="field.required"
                                    :placeholder="field.ui_settings?.placeholder"
                                    :options="field.ui_settings?.options"
                                    :min="field.constraints?.min"
                                    :max="field.constraints?.max"
                                    :step="field.constraints?.step"
                                    :pattern="field.constraints?.pattern"
                                />
                            </v-col>
                        </v-row>
                    </div>
                </v-form>
            </v-card-text>

            <v-card-actions class="pa-4 px-6">
                <v-spacer />
                <v-btn
                    :variant="buttonConfigs.text.variant"
                    color="mutedForeground"
                    :disabled="loading"
                    @click="closeDialog"
                >
                    {{ t('common.cancel') }}
                </v-btn>
                <v-btn
                    :color="buttonConfigs.primary.color"
                    :variant="buttonConfigs.primary.variant"
                    :loading="loading"
                    :disabled="!isValid"
                    @click="updateEntity"
                >
                    {{ t('common.save') }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { ref, computed, watch } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useFieldRendering } from '@/composables/useFieldRendering'
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import { getDialogMaxWidth, buttonConfigs } from '@/design-system/components'
    import type { DynamicEntity, EntityDefinition, UpdateEntityRequest } from '@/types/schemas'

    interface Props {
        modelValue: boolean
        entity: DynamicEntity | null
        entityDefinition: EntityDefinition | null
        loading?: boolean
    }

    interface Emits {
        (e: 'update:modelValue', value: boolean): void

        (e: 'update', data: UpdateEntityRequest): void
    }

    const props = withDefaults(defineProps<Props>(), {
        loading: false,
    })

    const emit = defineEmits<Emits>()

    const { t } = useTranslations()
    const { getFieldComponent, getFieldRules, parseJsonFieldValue, stringifyJsonFieldValue } =
        useFieldRendering()

    // Form state
    const form = ref()
    const isValid = ref(false)
    const formData = ref<UpdateEntityRequest>({
        data: {
            published: false,
        },
        parent_uuid: null,
    })
    const fieldErrors = ref<Record<string, string>>({})

    // Computed properties
    const dialogVisible = computed({
        get: () => props.modelValue,
        set: value => emit('update:modelValue', value),
    })

    const availableParents = computed((): Array<{ title: string; uuid: string }> => {
        if (!props.entity?.entity_type) {
            return []
        }

        // This would need to be populated with actual entities
        // For now, return empty array
        return []
    })

    const parentUuidValue = computed({
        get: () => formData.value.parent_uuid,
        set: (value: string | null | undefined) => {
            formData.value.parent_uuid = value ?? null
        },
    })

    // Methods
    const initializeFormData = () => {
        if (props.entity) {
            const data = { ...(props.entity.field_data || {}) }
            if (data.published === undefined) {
                data.published = false
            }

            // Stringify JSON fields for display in textareas
            if (props.entityDefinition) {
                for (const field of props.entityDefinition.fields) {
                    if (data[field.name] !== undefined) {
                        data[field.name] = stringifyJsonFieldValue(
                            data[field.name],
                            field.field_type
                        )
                    }
                }
            }

            formData.value = {
                data,
                parent_uuid: (props.entity.field_data?.parent_uuid as string) ?? null,
            }
            fieldErrors.value = {}
        }
    }

    const updateEntity = async () => {
        if (!form.value?.validate()) {
            return
        }

        // Clear any previous errors
        fieldErrors.value = {}

        // Parse JSON fields before sending
        const processedData: Record<string, unknown> = { ...formData.value.data }
        if (props.entityDefinition) {
            for (const field of props.entityDefinition.fields) {
                const fieldName = field.name
                if (processedData[fieldName] !== undefined) {
                    const { parsed, error } = parseJsonFieldValue(
                        processedData[fieldName],
                        field.field_type
                    )
                    if (error) {
                        fieldErrors.value[fieldName] = error
                    } else {
                        processedData[fieldName] = parsed
                    }
                }
            }
        }

        // If there were JSON parsing errors, don't submit
        if (Object.keys(fieldErrors.value).length > 0) {
            return
        }

        emit('update', {
            data: processedData,
            parent_uuid: formData.value.parent_uuid,
        })
    }

    const getFieldErrorMessages = (fieldName: string) => {
        const error = fieldErrors.value[fieldName]
        return error ? [error] : []
    }

    const setFieldErrors = (errors: Record<string, string>) => {
        fieldErrors.value = errors
    }

    // Expose method for parent to set field errors
    defineExpose({
        setFieldErrors,
    })

    const closeDialog = () => {
        dialogVisible.value = false
        formData.value = {
            data: {
                published: false,
            },
            parent_uuid: undefined,
        }
        fieldErrors.value = {}
    }

    // Watch for dialog visibility changes
    watch(dialogVisible, visible => {
        if (visible && props.entity) {
            initializeFormData()
        } else if (!visible) {
            closeDialog()
        }
    })

    // Watch for entity changes
    watch(
        () => props.entity,
        entity => {
            if (entity && dialogVisible.value) {
                initializeFormData()
            }
        }
    )

    // Watch formData and clear field errors when user edits fields
    watch(
        () => formData.value.data,
        () => {
            // Clear any field errors when user edits the form
            if (Object.keys(fieldErrors.value).length > 0) {
                fieldErrors.value = {}
            }
        },
        { deep: true }
    )
</script>
