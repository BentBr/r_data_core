<template>
    <v-container fluid>
        <v-row>
            <v-col cols="12">
                <v-card>
                    <v-card-title class="d-flex align-center justify-space-between pa-4">
                        <div class="d-flex align-center">
                            <v-icon
                                icon="mdi-file-tree"
                                class="mr-3"
                            />
                            <span class="text-h4">{{ t('navigation.entity_definitions') }}</span>
                        </div>
                        <v-btn
                            color="primary"
                            prepend-icon="mdi-plus"
                            @click="showCreateDialog = true"
                        >
                            {{ t('entity_definitions.create.button') }}
                        </v-btn>
                    </v-card-title>

                    <v-card-text>
                        <v-row>
                            <!-- Tree View -->
                            <v-col cols="4">
                                <EntityDefinitionTree
                                    :entity-definitions="entityDefinitions"
                                    :loading="loading"
                                    :expanded-groups="expandedGroups"
                                    @update:expanded-groups="updateExpandedGroups"
                                    @item-click="handleItemClick"
                                    @selection-change="handleTreeSelection"
                                />
                            </v-col>

                            <!-- Details Panel -->
                            <v-col cols="8">
                                <EntityDefinitionDetails
                                    :definition="selectedDefinition"
                                    :has-unsaved-changes="hasUnsavedChanges"
                                    :saving-changes="savingChanges"
                                    @edit="editDefinition"
                                    @delete="showDeleteDialog = true"
                                    @save-changes="saveChanges"
                                    @add-field="addField"
                                    @edit-field="editField"
                                    @remove-field="removeField"
                                />
                            </v-col>
                        </v-row>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>

        <!-- Dialogs -->
        <EntityDefinitionCreateDialog
            v-model="showCreateDialog"
            :loading="creating"
            @create="createEntityDefinition"
        />

        <EntityDefinitionEditDialog
            v-model="showEditDialog"
            :definition="selectedDefinition"
            :loading="updating"
            @update="updateEntityDefinition"
        />

        <FieldEditor
            v-model="showFieldEditor"
            :field="editingField"
            @save="handleFieldSave"
        />

        <DialogManager
            v-model="showDeleteDialog"
            :config="deleteDialogConfig"
            :loading="deleting"
            @confirm="deleteEntityDefinition"
        />

        <!-- Snackbar -->
        <SnackbarManager :snackbar="currentSnackbar" />
    </v-container>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted, onUnmounted } from 'vue'
    import { useAuthStore } from '@/stores/auth'
    import { typedHttpClient } from '@/api/typed-client'
    import { useTranslations } from '@/composables/useTranslations'
    import { useSnackbar } from '@/composables/useSnackbar'
    import type {
        EntityDefinition,
        CreateEntityDefinitionRequest,
        UpdateEntityDefinitionRequest,
        FieldDefinition,
        TreeNode,
    } from '@/types/schemas'
    import EntityDefinitionTree from '@/components/entity-definitions/EntityDefinitionTree.vue'
    import EntityDefinitionDetails from '@/components/entity-definitions/EntityDefinitionDetails.vue'
    import EntityDefinitionCreateDialog from '@/components/entity-definitions/EntityDefinitionCreateDialog.vue'
    import EntityDefinitionEditDialog from '@/components/entity-definitions/EntityDefinitionEditDialog.vue'
    import FieldEditor from '@/components/forms/FieldEditor.vue'
    import DialogManager from '@/components/common/DialogManager.vue'
    import SnackbarManager from '@/components/common/SnackbarManager.vue'

    const authStore = useAuthStore()
    const { t } = useTranslations()
    const { currentSnackbar, showSuccess, showError } = useSnackbar()

    // Reactive state
    const loading = ref(false)
    const entityDefinitions = ref<EntityDefinition[]>([])
    const selectedDefinition = ref<EntityDefinition | null>(null)
    const selectedItems = ref<string[]>([])
    const expandedGroups = ref<string[]>([])
    const originalDefinition = ref<EntityDefinition | null>(null)
    const error = ref('')

    // Dialog states
    const showCreateDialog = ref(false)
    const showEditDialog = ref(false)
    const showDeleteDialog = ref(false)
    const showFieldEditor = ref(false)
    const editingField = ref<FieldDefinition | undefined>(undefined)

    // Form states
    const creating = ref(false)
    const updating = ref(false)
    const deleting = ref(false)
    const savingChanges = ref(false)

    // Component lifecycle flag
    const isComponentMounted = ref(false)

    // Computed properties
    const hasUnsavedChanges = computed(() => {
        if (!selectedDefinition.value || !originalDefinition.value) {
            return false
        }

        // Simple field count comparison
        if (selectedDefinition.value.fields.length !== originalDefinition.value.fields.length) {
            return true
        }

        // Create maps for field comparison
        const currentFieldsMap = new Map<string, FieldDefinition>(
            selectedDefinition.value.fields.map(field => [field.name, field])
        )
        const originalFieldsMap = new Map<string, FieldDefinition>(
            originalDefinition.value.fields.map(field => [field.name, field])
        )

        // Check for added/removed fields
        const currentFieldNames = Array.from(currentFieldsMap.keys())
        const originalFieldNames = Array.from(originalFieldsMap.keys())

        // Check if any fields were added or removed
        for (const fieldName of currentFieldNames) {
            if (!originalFieldsMap.has(fieldName)) {
                return true // New field added
            }
        }

        for (const fieldName of originalFieldNames) {
            if (!currentFieldsMap.has(fieldName)) {
                return true // Field removed
            }
        }

        // Check if any existing fields were modified
        for (const fieldName of currentFieldNames) {
            const currentField = currentFieldsMap.get(fieldName)!
            const originalField = originalFieldsMap.get(fieldName)!

            if (
                currentField.name !== originalField.name ||
                currentField.display_name !== originalField.display_name ||
                currentField.field_type !== originalField.field_type ||
                currentField.required !== originalField.required ||
                currentField.indexed !== originalField.indexed ||
                currentField.filterable !== originalField.filterable ||
                currentField.description !== originalField.description ||
                JSON.stringify(currentField.constraints) !==
                    JSON.stringify(originalField.constraints) ||
                JSON.stringify(currentField.ui_settings) !==
                    JSON.stringify(originalField.ui_settings)
            ) {
                return true
            }
        }

        return false
    })

    const deleteDialogConfig = computed(() => ({
        title: t('entity_definitions.delete.title'),
        maxWidth: '500px',
        persistent: false,
    }))

    // Methods
    const sanitizeFields = (fields: FieldDefinition[]) => {
        return fields.map(field => ({
            ...field,
            constraints: field.constraints || {},
            ui_settings: field.ui_settings || {},
        }))
    }

    const loadEntityDefinitions = async () => {
        if (!isComponentMounted.value || !authStore.isAuthenticated) {
            return
        }

        loading.value = true
        error.value = ''

        try {
            const response = await typedHttpClient.getEntityDefinitions()
            // Sanitize fields to ensure constraints and ui_settings are always objects
            entityDefinitions.value = (response.data || []).map(definition => ({
                ...definition,
                fields: sanitizeFields(definition.fields),
            }))
        } catch (err) {
            console.error('Failed to load entity definitions:', err)
            error.value = err instanceof Error ? err.message : 'Failed to load entity definitions'
            // Don't clear items on error to maintain layout
        } finally {
            loading.value = false
        }
    }

    const updateExpandedGroups = (groups: string[]) => {
        expandedGroups.value = groups
    }

    const handleTreeSelection = (items: string[]) => {
        if (items.length > 0) {
            const selectedId = items[0]
            // Check if it's a group or actual entity definition
            if (selectedId.startsWith('group-')) {
                // It's a group, expand/collapse it
                if (expandedGroups.value.includes(selectedId)) {
                    expandedGroups.value = expandedGroups.value.filter(id => id !== selectedId)
                } else {
                    expandedGroups.value.push(selectedId)
                }
            } else {
                // It's an entity definition, select it
                const definition = entityDefinitions.value.find(d => d.uuid === selectedId)
                if (definition) {
                    selectedDefinition.value = definition
                    // Deep copy the definition including fields with sanitization
                    originalDefinition.value = {
                        ...definition,
                        fields: sanitizeFields(definition.fields.map(field => ({ ...field }))),
                    }
                    selectedItems.value = [selectedId]
                }
            }
        } else {
            selectedDefinition.value = null
        }
    }

    const handleItemClick = (item: TreeNode) => {
        if (item.entity_type === 'group') {
            // For groups, toggle expansion
            const groupId = item.id
            if (expandedGroups.value.includes(groupId)) {
                expandedGroups.value = expandedGroups.value.filter(id => id !== groupId)
            } else {
                expandedGroups.value.push(groupId)
            }
        } else {
            // For entity definitions, select them
            const definition = entityDefinitions.value.find(d => d.uuid === item.uuid)
            if (definition) {
                selectedDefinition.value = definition
                // Deep copy the definition including fields with sanitization
                originalDefinition.value = {
                    ...definition,
                    fields: sanitizeFields(definition.fields.map(field => ({ ...field }))),
                }
                selectedItems.value = [item.uuid!]
            }
        }
    }

    const saveChanges = async () => {
        if (!selectedDefinition.value || !originalDefinition.value) {
            return
        }

        savingChanges.value = true

        try {
            await typedHttpClient.updateEntityDefinition(selectedDefinition.value.uuid!, {
                entity_type: selectedDefinition.value.entity_type,
                display_name: selectedDefinition.value.display_name,
                description: selectedDefinition.value.description,
                group_name: selectedDefinition.value.group_name,
                allow_children: selectedDefinition.value.allow_children,
                icon: selectedDefinition.value.icon,
                fields: selectedDefinition.value.fields,
                published: selectedDefinition.value.published,
            })

            // Update original definition to reflect saved state
            originalDefinition.value = {
                ...selectedDefinition.value,
                fields: sanitizeFields(
                    selectedDefinition.value.fields.map(field => ({ ...field }))
                ),
            }

            showSuccess(t('entity_definitions.details.changes_saved'))
        } catch (err) {
            showError(
                err instanceof Error ? err.message : t('entity_definitions.details.save_error')
            )
        } finally {
            savingChanges.value = false
        }
    }

    const createEntityDefinition = async (data: CreateEntityDefinitionRequest) => {
        creating.value = true

        try {
            await typedHttpClient.createEntityDefinition(data)
            showCreateDialog.value = false

            // Reload the list
            await loadEntityDefinitions()

            showSuccess('Entity definition created successfully')
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : ''

            // Handle specific 409 conflict errors
            if (
                errorMessage.includes('409') ||
                errorMessage.includes('conflict') ||
                errorMessage.includes('already exists')
            ) {
                showError(t('entity_definitions.errors.entity_type_exists'))
            } else if (errorMessage.includes('422') || errorMessage.includes('validation')) {
                if (
                    errorMessage.includes('field name') ||
                    errorMessage.includes('invalid characters') ||
                    errorMessage.includes('reserved SQL keyword')
                ) {
                    showError(t('entity_definitions.errors.invalid_characters'))
                } else if (
                    errorMessage.includes('field exists') ||
                    errorMessage.includes('already exists')
                ) {
                    showError(t('entity_definitions.errors.field_name_exists'))
                } else {
                    showError(t('entity_definitions.errors.validation_failed'))
                }
            } else {
                showError(errorMessage || t('entity_definitions.create.error'))
            }
        } finally {
            creating.value = false
        }
    }

    const editDefinition = () => {
        showEditDialog.value = true
    }

    const updateEntityDefinition = async (data: UpdateEntityDefinitionRequest) => {
        if (!selectedDefinition.value) {
            return
        }

        updating.value = true

        try {
            await typedHttpClient.updateEntityDefinition(selectedDefinition.value.uuid!, data)

            // Update the selected definition
            selectedDefinition.value = {
                ...selectedDefinition.value,
                ...data,
            }

            // Update the list
            const index = entityDefinitions.value.findIndex(
                d => d.uuid === selectedDefinition.value?.uuid
            )
            if (index !== -1) {
                entityDefinitions.value[index] = selectedDefinition.value
            }

            showEditDialog.value = false

            showSuccess('Entity definition updated successfully')
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : ''

            // Handle specific 422 validation errors
            if (errorMessage.includes('422') || errorMessage.includes('validation')) {
                if (
                    errorMessage.includes('field name') ||
                    errorMessage.includes('invalid characters')
                ) {
                    showError(t('entity_definitions.errors.invalid_characters'))
                } else if (
                    errorMessage.includes('field exists') ||
                    errorMessage.includes('already exists')
                ) {
                    showError(t('entity_definitions.errors.field_name_exists'))
                } else if (
                    errorMessage.includes('entity type') ||
                    errorMessage.includes('type exists')
                ) {
                    showError(t('entity_definitions.errors.entity_type_exists'))
                } else {
                    showError(t('entity_definitions.errors.validation_failed'))
                }
            } else {
                showError(errorMessage || t('entity_definitions.edit.error'))
            }
        } finally {
            updating.value = false
        }
    }

    const deleteEntityDefinition = async () => {
        if (!selectedDefinition.value) {
            return
        }

        deleting.value = true

        try {
            await typedHttpClient.deleteEntityDefinition(selectedDefinition.value.uuid!)

            // Remove from list
            entityDefinitions.value = entityDefinitions.value.filter(
                d => d.uuid !== selectedDefinition.value?.uuid
            )
            selectedDefinition.value = null
            selectedItems.value = []

            showDeleteDialog.value = false

            showSuccess(t('entity_definitions.delete.success'))
        } catch (err) {
            showError(err instanceof Error ? err.message : t('entity_definitions.delete.error'))
        } finally {
            deleting.value = false
        }
    }

    const addField = () => {
        editingField.value = undefined
        showFieldEditor.value = true
    }

    const editField = (field: FieldDefinition) => {
        editingField.value = field
        showFieldEditor.value = true
    }

    const removeField = (field: FieldDefinition) => {
        if (selectedDefinition.value) {
            const index = selectedDefinition.value.fields.findIndex(f => f.name === field.name)
            if (index !== -1) {
                selectedDefinition.value.fields.splice(index, 1)
            }
        }
    }

    const handleFieldSave = (field: FieldDefinition) => {
        // Ensure constraints and ui_settings are always objects, not null
        const sanitizedField = {
            ...field,
            constraints: field.constraints || {},
            ui_settings: field.ui_settings || {},
        }

        // Working with selected definition
        if (editingField.value) {
            // Editing existing field
            const index = selectedDefinition.value?.fields.findIndex(
                f => f.name === editingField.value?.name
            )
            if (index !== -1 && index !== undefined && selectedDefinition.value) {
                selectedDefinition.value.fields[index] = sanitizedField
            }
        } else {
            // Adding new field
            if (selectedDefinition.value) {
                selectedDefinition.value.fields.push(sanitizedField)
            }
        }

        // Don't update originalDefinition here - we want hasUnsavedChanges to detect the changes
    }

    // Lifecycle
    onMounted(() => {
        isComponentMounted.value = true
        loadEntityDefinitions()
    })

    onUnmounted(() => {
        isComponentMounted.value = false
    })
</script>

<style scoped>
    /* Ensure stable layout to prevent scrollbar shifts */
    .v-container {
        min-height: calc(100vh - 64px - 32px); /* Account for app bar and padding */
        overflow-x: hidden;
    }

    /* Ensure card containers have stable dimensions */
    .v-card {
        min-height: 400px;
    }

    /* Ensure tree and details panels have stable heights */
    .v-col {
        min-height: 500px;
    }
</style>
