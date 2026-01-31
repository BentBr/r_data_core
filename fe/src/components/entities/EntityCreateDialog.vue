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

                    <!-- Path Field with Combobox (allows custom input) -->
                    <v-combobox
                        v-model="pathValue"
                        :items="pathSuggestions"
                        item-title="path"
                        item-value="path"
                        :label="t('entities.create.path_label')"
                        :hint="pathHint"
                        :loading="pathLoading"
                        :error-messages="fieldErrors.path ? [fieldErrors.path] : []"
                        persistent-hint
                        clearable
                        no-filter
                        class="mb-4"
                        @update:search="onPathInput"
                        @update:model-value="onPathSelected"
                    >
                        <template #item="{ item, props: itemProps }">
                            <v-list-item
                                v-bind="itemProps"
                                :subtitle="
                                    item.raw.entity_type
                                        ? `${item.raw.entity_type} - ${item.raw.name}`
                                        : item.raw.name
                                "
                            >
                                <template #prepend>
                                    <SmartIcon
                                        :icon="item.raw.kind === 'folder' ? 'folder' : 'file'"
                                        size="sm"
                                    />
                                </template>
                            </v-list-item>
                        </template>
                        <template #no-data>
                            <v-list-item v-if="pathLoading">
                                <v-list-item-title>
                                    {{ t('table.loading') }}
                                </v-list-item-title>
                            </v-list-item>
                            <v-list-item v-else-if="pathSearchTerm && pathSearchTerm.length > 1">
                                <v-list-item-title>
                                    {{ t('entities.tree.no_entities') }}
                                </v-list-item-title>
                            </v-list-item>
                        </template>
                    </v-combobox>

                    <!-- Parent Entity Selection with Autocomplete -->
                    <v-autocomplete
                        :model-value="parentUuidValue"
                        :items="formattedParentSuggestions"
                        item-title="title"
                        item-value="value"
                        :label="t('entities.create.parent_label')"
                        :loading="parentLoading"
                        clearable
                        no-filter
                        class="mb-4"
                        @click:control="onParentDropdownClick"
                        @update:search="onParentSearch"
                        @update:model-value="onParentSelect"
                    >
                        <template #selection>
                            {{ selectedParentDisplay }}
                        </template>
                        <template #item="{ item, props: itemProps }">
                            <v-list-item
                                v-bind="itemProps"
                                :subtitle="item.raw.subtitle"
                            >
                                <template #prepend>
                                    <SmartIcon
                                        icon="file"
                                        size="sm"
                                    />
                                </template>
                            </v-list-item>
                        </template>
                        <template #no-data>
                            <v-list-item>
                                <v-list-item-title>
                                    {{
                                        parentLoading
                                            ? t('table.loading')
                                            : t('entities.tree.no_entities')
                                    }}
                                </v-list-item-title>
                            </v-list-item>
                        </template>
                    </v-autocomplete>

                    <!-- Published Switch - Now below parent -->
                    <v-switch
                        v-model="formData.data.published"
                        :label="t('entities.create.published_label')"
                        :hint="t('entities.create.published_hint')"
                        color="success"
                        inset
                        persistent-hint
                        class="mb-4"
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
    import { ValidationError, typedHttpClient } from '@/api/typed-client'
    import { useFieldRendering } from '@/composables/useFieldRendering'

    // Type for browse node from API
    interface BrowseNode {
        kind: 'folder' | 'file'
        name: string
        path: string
        entity_uuid?: string | null
        entity_type?: string | null
        has_children?: boolean | null
        published: boolean
    }

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
    const { getFieldComponent, getFieldRules, parseJsonFieldValue } = useFieldRendering()

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

    // Path autocomplete state
    const pathSuggestions = ref<BrowseNode[]>([])
    const pathLoading = ref(false)
    const pathSearchTerm = ref('')
    let pathDebounceTimer: ReturnType<typeof setTimeout> | null = null

    // Parent autocomplete state
    const parentSuggestions = ref<BrowseNode[]>([])
    const parentLoading = ref(false)
    let parentDebounceTimer: ReturnType<typeof setTimeout> | null = null

    // Store selected parent display text to avoid showing UUID when item is not in suggestions
    const selectedParentDisplay = ref<string | null>(null)

    // Flag to prevent clearing parent when path is set programmatically
    const pathSetByParent = ref(false)

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

    const isRootPath = computed(() => formData.value.data.path === '/')

    // Check if the current path is a new folder (not matching any existing entity)
    const isCreatingNewFolder = computed(() => {
        const path = formData.value.data.path as string
        if (!path || path === '/') {
            return false
        }
        // Check if path matches any entity in suggestions
        const matchesEntity = pathSuggestions.value.some(node => node.path === path)
        return !matchesEntity
    })

    // Extract folder name from path (last segment)
    const newFolderName = computed(() => {
        const path = formData.value.data.path as string
        if (!path || path === '/') {
            return ''
        }
        // Remove trailing slash if present and get last segment
        const cleanPath = path.endsWith('/') ? path.slice(0, -1) : path
        const segments = cleanPath.split('/')
        return segments[segments.length - 1] || ''
    })

    const pathHint = computed(() => {
        if (isRootPath.value) {
            return t('entities.create.root_folder_hint')
        }
        if (isCreatingNewFolder.value && newFolderName.value) {
            return t('entities.create.new_folder_hint', { folder: newFolderName.value })
        }
        return t('entities.create.path_hint')
    })

    const formattedParentSuggestions = computed(() => {
        return parentSuggestions.value.map(node => ({
            title: node.path,
            value: node.entity_uuid,
            subtitle: node.entity_type ? `${node.entity_type} - ${node.name}` : node.name,
            raw: node,
        }))
    })

    const parentUuidValue = computed({
        get: () => formData.value.parent_uuid,
        set: (value: string | null | undefined) => {
            formData.value.parent_uuid = value ?? null
        },
    })

    // Computed for path combobox - handles both string input and object selection
    const pathValue = computed({
        get: () => formData.value.data.path as string,
        set: (value: string | BrowseNode | null | undefined) => {
            if (value === null || value === undefined) {
                formData.value.data.path = '/'
            } else if (typeof value === 'string') {
                formData.value.data.path = value
            } else {
                // It's a BrowseNode object
                formData.value.data.path = value.path
            }
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

        // Clear parent and suggestions
        formData.value.parent_uuid = null
        selectedParentDisplay.value = null
        pathSuggestions.value = []
        parentSuggestions.value = []

        // Initialize with default values from entity definition
        if (selectedEntityDefinition.value) {
            for (const field of selectedEntityDefinition.value.fields) {
                if (field.ui_settings?.default !== undefined) {
                    formData.value.data[field.name] = field.ui_settings.default
                }
            }
        }
    }

    const onPathInput = (value: string | null) => {
        pathSearchTerm.value = value ?? ''

        // Clear previous debounce timer
        if (pathDebounceTimer) {
            clearTimeout(pathDebounceTimer)
        }

        // Don't search for empty or root path
        if (!value || value === '/' || value.length < 2) {
            pathSuggestions.value = []
            return
        }

        // Debounced search
        pathDebounceTimer = setTimeout(() => {
            void (async () => {
                pathLoading.value = true
                try {
                    const result = await typedHttpClient.searchEntitiesByPath(value, 10)
                    pathSuggestions.value = result.data
                } catch (error) {
                    console.error('Path search error:', error)
                    pathSuggestions.value = []
                } finally {
                    pathLoading.value = false
                }
            })()
        }, 350)
    }

    const onPathSelected = (value: unknown) => {
        // Handle both string (custom input) and object (selected from list) values
        let pathStr: string | null = null
        let selectedNode: BrowseNode | undefined

        if (value === null || value === undefined) {
            pathStr = null
        } else if (typeof value === 'string') {
            pathStr = value
            selectedNode = pathSuggestions.value.find(node => node.path === pathStr)
        } else {
            // It's a BrowseNode object selected from the list
            const node = value as BrowseNode
            pathStr = node.path
            selectedNode = node
        }

        // If path was not set by parent selection, clear parent
        if (!pathSetByParent.value && formData.value.parent_uuid) {
            formData.value.parent_uuid = null
            selectedParentDisplay.value = null
        }

        // If selected value corresponds to an entity, set it as parent
        if (selectedNode?.entity_uuid) {
            pathSetByParent.value = true
            formData.value.parent_uuid = selectedNode.entity_uuid
            selectedParentDisplay.value = selectedNode.path
        }

        // Reset the flag after a tick
        setTimeout(() => {
            pathSetByParent.value = false
        }, 0)
    }

    const onParentDropdownClick = async () => {
        if (parentLoading.value) {
            return
        }

        parentLoading.value = true
        try {
            const currentPath = (formData.value.data.path as string) || '/'
            const result = await typedHttpClient.browseByPath(currentPath, 10)
            // Filter to only show files (entities), not folders
            parentSuggestions.value = result.data.filter(
                node => node.kind === 'file' && node.entity_uuid
            )
        } catch (error) {
            console.error('Parent load error:', error)
            parentSuggestions.value = []
        } finally {
            parentLoading.value = false
        }
    }

    const onParentSearch = (searchTerm: string | null) => {
        // Clear previous debounce timer
        if (parentDebounceTimer) {
            clearTimeout(parentDebounceTimer)
        }

        if (!searchTerm) {
            void onParentDropdownClick()
            return
        }

        // Debounced search
        parentDebounceTimer = setTimeout(() => {
            void (async () => {
                parentLoading.value = true
                try {
                    const result = await typedHttpClient.searchEntitiesByPath(searchTerm, 10)
                    // Filter to only show files (entities), not folders
                    parentSuggestions.value = result.data.filter(
                        node => node.kind === 'file' && node.entity_uuid
                    )
                } catch (error) {
                    console.error('Parent search error:', error)
                    parentSuggestions.value = []
                } finally {
                    parentLoading.value = false
                }
            })()
        }, 350)
    }

    const onParentSelect = (uuid: string | null) => {
        formData.value.parent_uuid = uuid

        if (uuid) {
            // Find the selected parent in suggestions
            const selected = parentSuggestions.value.find(s => s.entity_uuid === uuid)
            if (selected) {
                pathSetByParent.value = true
                formData.value.data.path = selected.path
                selectedParentDisplay.value = selected.path
            }
        } else {
            selectedParentDisplay.value = null
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

        // Parse JSON fields before sending
        const processedData: Record<string, unknown> = { ...formData.value.data }
        if (selectedEntityDefinition.value) {
            for (const field of selectedEntityDefinition.value.fields) {
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

        emit('create', {
            entity_type: formData.value.entity_type,
            data: processedData,
            parent_uuid: formData.value.parent_uuid,
        })
    }

    // Expose method for parent to set field errors
    defineExpose({
        setFieldErrors,
    })

    const closeDialog = () => {
        // Clear debounce timers
        if (pathDebounceTimer) {
            clearTimeout(pathDebounceTimer)
            pathDebounceTimer = null
        }
        if (parentDebounceTimer) {
            clearTimeout(parentDebounceTimer)
            parentDebounceTimer = null
        }

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
        pathSuggestions.value = []
        parentSuggestions.value = []
        pathSearchTerm.value = ''
        pathSetByParent.value = false
        selectedParentDisplay.value = null
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

    // Watch defaultParent prop - if provided, prefill parent_uuid and path
    watch(
        () => props.defaultParent,
        async newParent => {
            // Only prefill if dialog is open AND entity_type is already selected
            if (newParent && dialogVisible.value && formData.value.entity_type) {
                pathSetByParent.value = true
                formData.value.parent_uuid = (newParent.field_data?.uuid as string) ?? null

                // Also update the path based on parent
                const parentPath = newParent.field_data?.path as string | undefined
                const parentKey = newParent.field_data?.entity_key as string | undefined

                if (parentPath && parentKey) {
                    // Set path to parent.path + '/' + parent.entity_key
                    const fullPath = parentPath.endsWith('/')
                        ? `${parentPath}${parentKey}`
                        : `${parentPath}/${parentKey}`
                    formData.value.data.path = fullPath
                    selectedParentDisplay.value = fullPath
                }

                setTimeout(() => {
                    pathSetByParent.value = false
                }, 0)
            }
        }
    )

    // Watch for when dialog opens to potentially set default parent if already selected
    watch(dialogVisible, async visible => {
        if (visible && props.defaultParent && formData.value.entity_type) {
            pathSetByParent.value = true
            formData.value.parent_uuid = (props.defaultParent.field_data?.uuid as string) ?? null

            const parentPath = props.defaultParent.field_data?.path as string | undefined
            const parentKey = props.defaultParent.field_data?.entity_key as string | undefined

            if (parentPath && parentKey) {
                const fullPath = parentPath.endsWith('/')
                    ? `${parentPath}${parentKey}`
                    : `${parentPath}/${parentKey}`
                formData.value.data.path = fullPath
                selectedParentDisplay.value = fullPath
            }

            setTimeout(() => {
                pathSetByParent.value = false
            }, 0)
        }
    })
</script>
