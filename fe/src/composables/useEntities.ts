import { ref, computed } from 'vue'
import { typedHttpClient, ValidationError } from '@/api/typed-client'
import { useErrorHandler } from './useErrorHandler'
import { useTranslations } from './useTranslations'
import type {
    DynamicEntity,
    EntityDefinition,
    CreateEntityRequest,
    UpdateEntityRequest,
} from '@/types/schemas'

export function useEntities() {
    const { handleError, handleSuccess } = useErrorHandler()
    const { t } = useTranslations()

    // State
    const entities = ref<DynamicEntity[]>([])
    const entityDefinitions = ref<EntityDefinition[]>([])
    const selectedEntity = ref<DynamicEntity | null>(null)
    const loading = ref(false)
    const creating = ref(false)
    const updating = ref(false)
    const deleting = ref(false)
    const selectedItems = ref<string[]>([])
    const expandedItems = ref<string[]>([])
    const error = ref('')

    /**
     * Load entity definitions
     */
    const loadEntityDefinitions = async (): Promise<void> => {
        try {
            const response = await typedHttpClient.getEntityDefinitions()
            entityDefinitions.value = response.data || []
        } catch (err) {
            handleError(err, 'Failed to load entity definitions')
            error.value = err instanceof Error ? err.message : 'Failed to load entity definitions'
        }
    }

    /**
     * Get entity by type and UUID
     */
    const getEntity = async (entityType: string, uuid: string): Promise<DynamicEntity | null> => {
        loading.value = true
        try {
            const entity = await typedHttpClient.getEntity(entityType, uuid)
            selectedEntity.value = entity
            return entity
        } catch (err) {
            handleError(err, 'Failed to load entity')
            return null
        } finally {
            loading.value = false
        }
    }

    /**
     * Create an entity
     */
    const createEntity = async (
        data: CreateEntityRequest,
        onSuccess?: () => void
    ): Promise<DynamicEntity | null> => {
        creating.value = true
        try {
            await typedHttpClient.createEntity(data.entity_type, data)
            handleSuccess(t('entities.create.success') || 'Entity created successfully')
            onSuccess?.()
            return null // Success
        } catch (err) {
            if (err instanceof ValidationError) {
                // Return validation errors for the UI to handle
                throw err
            }
            handleError(err, t('entities.create.error') || 'Failed to create entity')
            return null
        } finally {
            creating.value = false
        }
    }

    /**
     * Update an entity
     */
    const updateEntity = async (
        entityType: string,
        uuid: string,
        data: UpdateEntityRequest
    ): Promise<boolean> => {
        updating.value = true
        try {
            await typedHttpClient.updateEntity(entityType, uuid, data)

            // Update the selected entity
            if (selectedEntity.value) {
                selectedEntity.value = {
                    ...selectedEntity.value,
                    ...data,
                } as DynamicEntity
            }

            handleSuccess(t('entities.update.success') || 'Entity updated successfully')
            return true
        } catch (err) {
            handleError(err, t('entities.update.error') || 'Failed to update entity')
            return false
        } finally {
            updating.value = false
        }
    }

    /**
     * Delete an entity
     */
    const deleteEntity = async (entityType: string, uuid: string): Promise<boolean> => {
        deleting.value = true
        try {
            await typedHttpClient.deleteEntity(entityType, uuid)

            // Remove from list
            entities.value = entities.value.filter(e => e.field_data?.uuid !== uuid)
            selectedEntity.value = null
            selectedItems.value = []

            handleSuccess(t('entities.delete.success') || 'Entity deleted successfully')
            return true
        } catch (err) {
            handleError(err, t('entities.delete.error') || 'Failed to delete entity')
            return false
        } finally {
            deleting.value = false
        }
    }

    /**
     * Get the entity definition for a specific entity
     */
    const getEntityDefinition = (entityType: string): EntityDefinition | null => {
        return entityDefinitions.value.find(def => def.entity_type === entityType) ?? null
    }

    /**
     * Computed: selected entity's definition
     */
    const selectedEntityDefinition = computed(() => {
        if (!selectedEntity.value) {
            return null
        }
        return getEntityDefinition(selectedEntity.value.entity_type)
    })

    /**
     * Computed: selected entity UUID
     */
    const selectedEntityUuid = computed(() => {
        return selectedEntity.value?.field_data?.uuid ?? ''
    })

    return {
        // State
        entities,
        entityDefinitions,
        selectedEntity,
        loading,
        creating,
        updating,
        deleting,
        selectedItems,
        expandedItems,
        error,

        // Getters
        selectedEntityDefinition,
        selectedEntityUuid,

        // Methods
        loadEntityDefinitions,
        getEntity,
        createEntity,
        updateEntity,
        deleteEntity,
        getEntityDefinition,
    }
}
