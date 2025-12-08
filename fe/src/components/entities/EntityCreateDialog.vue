<template>
    <v-dialog
        v-model="dialogVisible"
        :max-width="getDialogMaxWidth('form')"
        persistent
    >
        <v-card>
            <v-card-title class="d-flex align-center pa-6">
                <SmartIcon
                    icon="plus"
                    size="md"
                    class="mr-3"
                />
                {{ t('entities.create.title') }}
            </v-card-title>

            <v-card-text class="pa-6">
                <v-form
                    ref="form"
                    v-model="isValid"
                >
                    <!-- Entity Type Selection (First) -->
                    <v-select
                        v-model="formData.entity_type"
                        :items="availableEntityTypes"
                        item-title="display_name"
                        item-value="entity_type"
                        :label="t('entities.create.entity_type_label')"
                        :rules="[v => !!v ?? t('entities.create.entity_type_required')]"
                        required
                        class="mb-4"
                        @update:model-value="onEntityTypeChange"
                    />

                    <!-- Key (mandatory) -->
                    <v-text-field
                        v-model="formData.data.entity_key"
                        :label="t('entities.create.key_label')"
                        :rules="[
                            (v: string | null | undefined) =>
                                !!v ?? t('entities.create.key_required'),
                        ]"
                        :error-messages="fieldErrors.entity_key ? [fieldErrors.entity_key] : []"
                        required
                        class="mb-4"
                    />

                    <!-- Optional Path -->
                    <v-text-field
                        v-model="formData.data.path"
                        :label="t('entities.create.path_label')"
                        hint="/ for root or /folder/subfolder"
                        :error-messages="fieldErrors.path ? [fieldErrors.path] : []"
                        persistent-hint
                        class="mb-4"
                    />

                    <!-- Published Switch -->
                    <v-switch
                        v-model="formData.data.published"
                        :label="t('entities.create.published_label')"
                        :hint="t('entities.create.published_hint')"
                        color="success"
                        inset
                        persistent-hint
                        class="mb-4"
                    />

                    <!-- Parent Entity Selection -->
                    <v-select
                        v-model="parentUuidValue"
                        :items="availableParents"
                        item-title="title"
                        item-value="uuid"
                        :label="t('entities.create.parent_label')"
                        clearable
                        class="mt-4"
                    />

                    <!-- Dynamic Fields based on Entity Definition -->
                    <div
                        v-if="selectedEntityDefinition"
                        class="mt-4"
                    >
                        <h4 class="text-subtitle-1 mb-3">{{ t('entities.create.data_label') }}</h4>

                        <v-row>
                            <v-col
                                v-for="field in selectedEntityDefinition.fields"
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
                    @click="createEntity"
                >
                    {{ t('entities.create.create_button') }}
                </v-btn>
            </v-card-actions>
        </v-card>
    </v-dialog>
</template>

<script setup lang="ts">
    import { ref, computed, watch } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import SmartIcon from '@/components/common/SmartIcon.vue'
    import { getDialogMaxWidth, buttonConfigs } from '@/design-system/components'
    import type { EntityDefinition, CreateEntityRequest, DynamicEntity } from '@/types/schemas'
    import { ValidationError } from '@/api/typed-client'
    import { useFieldRendering } from '@/composables/useFieldRendering'

    interface Props {
        modelValue: boolean
        entityDefinitions: EntityDefinition[]
        loading?: boolean
        defaultParent?: DynamicEntity | null
    }

    interface Emits {
        (e: 'update:modelValue', value: boolean): void
        (e: 'create', data: CreateEntityRequest): void
        (e: 'error', error: Error | ValidationError): void
    }

    const props = withDefaults(defineProps<Props>(), {
        loading: false,
    })

    const emit = defineEmits<Emits>()

    const { t } = useTranslations()
    const { getFieldComponent, getFieldRules } = useFieldRendering()

    // Form state
    const form = ref()
    const isValid = ref(false)
    const formData = ref<CreateEntityRequest>({
        entity_type: '',
        data: {
            entity_key: '',
            path: '/',
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

    const availableEntityTypes = computed(() => {
        return props.entityDefinitions
            .filter(def => def.published)
            .map(def => ({
                entity_type: def.entity_type,
                display_name: def.display_name,
            }))
    })

    const selectedEntityDefinition = computed(() => {
        return props.entityDefinitions.find(def => def.entity_type === formData.value.entity_type)
    })

    const availableParents = computed(() => {
        if (!formData.value.entity_type) {
            return []
        }

        // This would need to be populated with actual entities
        // For now, return empty array
        return []
    })

    const parentUuidValue = computed({
        get: () => formData.value.parent_uuid ?? null,
        set: (value: string | null) => {
            formData.value.parent_uuid = value
        },
    })

    // Methods
    const onEntityTypeChange = () => {
        // Reset form data when entity type changes
        formData.value.data = {}
        // Keep key and path fields visible
        formData.value.data.entity_key = ''
        formData.value.data.path = '/'
        formData.value.data.published = false

        // Initialize with default values from entity definition
        if (selectedEntityDefinition.value) {
            for (const field of selectedEntityDefinition.value.fields) {
                if (field.ui_settings?.default !== undefined) {
                    formData.value.data[field.name] = field.ui_settings.default
                }
            }
        }
    }

    const getFieldErrorMessages = (fieldName: string) => {
        const error = fieldErrors.value[fieldName]
        return error ? [error] : []
    }

    const setFieldErrors = (errors: Record<string, string>) => {
        fieldErrors.value = errors
    }

    const createEntity = async () => {
        if (!form.value?.validate()) {
            return
        }

        // Clear any previous errors
        fieldErrors.value = {}

        emit('create', {
            entity_type: formData.value.entity_type,
            data: formData.value.data,
            parent_uuid: formData.value.parent_uuid,
        })
    }

    // Expose method for parent to set field errors
    defineExpose({
        setFieldErrors,
    })

    const closeDialog = () => {
        dialogVisible.value = false
        fieldErrors.value = {}
        formData.value = {
            entity_type: '',
            data: {
                entity_key: '',
                path: '/',
                published: false,
            },
            parent_uuid: null,
        }
    }

    // Watch for dialog visibility changes
    watch(dialogVisible, visible => {
        if (!visible) {
            closeDialog()
        }
    })

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

    // Watch path changes - if user manually edits path, clear parent
    watch(
        () => formData.value.data.path,
        (newPath, oldPath) => {
            // Only clear parent if path changed manually (not when setting based on parent)
            if (oldPath !== undefined && formData.value.parent_uuid) {
                // User is manually editing the path, so clear the parent
                formData.value.parent_uuid = null
            }
        }
    )

    // Watch defaultParent prop - if provided, prefill parent_uuid and path
    watch(
        () => props.defaultParent,
        async newParent => {
            // Only prefill if dialog is open AND entity_type is already selected
            if (newParent && dialogVisible.value && formData.value.entity_type) {
                formData.value.parent_uuid = newParent.field_data?.uuid ?? null

                // Also update the path based on parent
                const parentPath = newParent.field_data?.path as string | undefined
                const parentKey = newParent.field_data?.entity_key as string | undefined

                if (parentPath && parentKey) {
                    // Set path to parent.path + '/' + parent.entity_key
                    formData.value.data.path = parentPath.endsWith('/')
                        ? `${parentPath}${parentKey}`
                        : `${parentPath}/${parentKey}`
                }
            }
        }
    )

    // Watch for when dialog opens to potentially set default parent if already selected
    watch(dialogVisible, async visible => {
        if (visible && props.defaultParent && formData.value.entity_type) {
            formData.value.parent_uuid = props.defaultParent.field_data?.uuid ?? null

            const parentPath = props.defaultParent.field_data?.path as string | undefined
            const parentKey = props.defaultParent.field_data?.entity_key as string | undefined

            if (parentPath && parentKey) {
                formData.value.data.path = parentPath.endsWith('/')
                    ? `${parentPath}${parentKey}`
                    : `${parentPath}/${parentKey}`
            }
        }
    })

    // Watch parent_uuid changes to update path
    watch(
        () => formData.value.parent_uuid,
        async newParentUuid => {
            if (newParentUuid && formData.value.entity_type) {
                try {
                    // Fetch parent entity to get its path and entity_key
                    const parentEntity = await typedHttpClient.getEntity(
                        formData.value.entity_type,
                        newParentUuid
                    )

                    const parentPath = parentEntity.field_data?.path as string | undefined
                    const parentKey = parentEntity.field_data?.entity_key as string | undefined

                    if (parentPath && parentKey) {
                        // Set path to parent.path + '/' + parent.entity_key
                        formData.value.data.path = parentPath.endsWith('/')
                            ? `${parentPath}${parentKey}`
                            : `${parentPath}/${parentKey}`
                    }
                } catch (error) {
                    console.error('Error fetching parent entity:', error)
                }
            }
        }
    )
</script>
