import { ref, computed } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { useErrorHandler } from './useErrorHandler'
import { useTranslations } from './useTranslations'
import type {
    EntityDefinition,
    CreateEntityDefinitionRequest,
    UpdateEntityDefinitionRequest,
    FieldDefinition,
} from '@/types/schemas'

export function useEntityDefinitions() {
    const { handleError, handleSuccess } = useErrorHandler()
    const { t } = useTranslations()

    // State
    const entityDefinitions = ref<EntityDefinition[]>([])
    const selectedDefinition = ref<EntityDefinition | null>(null)
    const originalDefinition = ref<EntityDefinition | null>(null)
    const selectedItems = ref<string[]>([])
    const expandedGroups = ref<string[]>([])
    const loading = ref(false)
    const creating = ref(false)
    const updating = ref(false)
    const deleting = ref(false)
    const savingChanges = ref(false)
    const error = ref('')

    /**
     * Sanitize fields to ensure constraints and ui_settings are always objects
     */
    const sanitizeFields = (fields: FieldDefinition[]): FieldDefinition[] => {
        return fields.map(field => ({
            ...field,
            constraints: field.constraints ?? {},
            ui_settings: field.ui_settings ?? {},
        }))
    }

    /**
     * Load entity definitions
     */
    const loadEntityDefinitions = async (): Promise<void> => {
        loading.value = true
        error.value = ''

        try {
            const response = await typedHttpClient.getEntityDefinitions()
            entityDefinitions.value = (response.data ?? []).map(definition => ({
                ...definition,
                fields: sanitizeFields(definition.fields),
            }))
        } catch (err) {
            console.error('Failed to load entity definitions:', err)
            error.value = err instanceof Error ? err.message : 'Failed to load entity definitions'
            handleError(err, 'Failed to load entity definitions')
        } finally {
            loading.value = false
        }
    }

    /**
     * Create an entity definition
     */
    const createEntityDefinition = async (
        data: CreateEntityDefinitionRequest
    ): Promise<boolean> => {
        creating.value = true

        try {
            await typedHttpClient.createEntityDefinition(data)

            // Reload the list
            await loadEntityDefinitions()

            handleSuccess(
                t('entity_definitions.create.success') ?? 'Entity definition created successfully'
            )
            return true
        } catch (err) {
            handleError(
                err,
                t('entity_definitions.create.error') || 'Failed to create entity definition'
            )
            return false
        } finally {
            creating.value = false
        }
    }

    /**
     * Update an entity definition
     */
    const updateEntityDefinition = async (
        data: UpdateEntityDefinitionRequest
    ): Promise<boolean> => {
        if (!selectedDefinition.value) {
            return false
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

            handleSuccess(
                t('entity_definitions.update.success') || 'Entity definition updated successfully'
            )
            return true
        } catch (err) {
            handleError(
                err,
                t('entity_definitions.update.error') || 'Failed to update entity definition'
            )
            return false
        } finally {
            updating.value = false
        }
    }

    /**
     * Delete an entity definition
     */
    const deleteEntityDefinition = async (): Promise<boolean> => {
        if (!selectedDefinition.value) {
            return false
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

            handleSuccess(
                t('entity_definitions.delete.success') || 'Entity definition deleted successfully'
            )
            return true
        } catch (err) {
            handleError(
                err,
                t('entity_definitions.delete.error') || 'Failed to delete entity definition'
            )
            return false
        } finally {
            deleting.value = false
        }
    }

    /**
     * Select an entity definition
     */
    const selectDefinition = (definition: EntityDefinition): void => {
        selectedDefinition.value = definition
        originalDefinition.value = {
            ...definition,
            fields: sanitizeFields(definition.fields.map(field => ({ ...field }))),
        }
    }

    /**
     * Computed: Check if there are unsaved changes
     */
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

    /**
     * Save changes to the selected definition
     */
    const saveChanges = async (): Promise<void> => {
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

            handleSuccess(
                t('entity_definitions.details.changes_saved') || 'Changes saved successfully'
            )
        } catch (err) {
            handleError(err, t('entity_definitions.details.save_error') || 'Failed to save changes')
        } finally {
            savingChanges.value = false
        }
    }

    return {
        // State
        entityDefinitions,
        selectedDefinition,
        originalDefinition,
        selectedItems,
        expandedGroups,
        loading,
        creating,
        updating,
        deleting,
        savingChanges,
        error,

        // Getters
        hasUnsavedChanges,

        // Methods
        loadEntityDefinitions,
        createEntityDefinition,
        updateEntityDefinition,
        deleteEntityDefinition,
        selectDefinition,
        saveChanges,
        sanitizeFields,
    }
}
