<template>
    <div>
        <PageLayout>
            <template #actions>
                <v-btn
                    color="secondary"
                    variant="outlined"
                    :loading="loading"
                    @click="loadEntities"
                >
                    <template #prepend>
                        <SmartIcon
                            icon="refresh-cw"
                            size="sm"
                        />
                    </template>
                    {{ t('common.refresh') }}
                </v-btn>
                <v-btn
                    v-if="canCreateEntity"
                    color="primary"
                    variant="flat"
                    @click="showCreateDialog = true"
                >
                    <template #prepend>
                        <SmartIcon
                            icon="plus"
                            size="sm"
                        />
                    </template>
                    {{ t('entities.create.button') }}
                </v-btn>
            </template>
            <v-row>
                <!-- Tree View -->
                <v-col cols="4">
                    <EntityTree
                        ref="entityTreeRef"
                        :root-path="'/'"
                        :loading="loading"
                        :expanded-items="expandedItems"
                        :refresh-key="treeRefreshKey"
                        :entity-definitions="entityDefinitions"
                        @update:expanded-items="updateExpandedItems"
                        @item-click="handleItemClick"
                        @selection-change="handleTreeSelection"
                    />
                </v-col>

                <!-- Details Panel -->
                <v-col cols="8">
                    <EntityDetails
                        :entity="selectedEntity"
                        :entity-definition="selectedEntityDefinition"
                        @edit="editEntity"
                        @delete="showDeleteDialog = true"
                    />
                </v-col>
            </v-row>

            <!-- Dialogs -->
            <EntityCreateDialog
                ref="createDialogRef"
                v-model="showCreateDialog"
                :entity-definitions="entityDefinitions"
                :loading="creating"
                :default-parent="selectedEntity"
                @create="createEntity"
            />

            <EntityEditDialog
                v-model="showEditDialog"
                :entity="selectedEntity"
                :entity-definition="selectedEntityDefinition"
                :loading="updating"
                @update="updateEntity"
            />

            <DialogManager
                v-model="showDeleteDialog"
                :config="deleteDialogConfig"
                :loading="deleting"
                @confirm="deleteEntity"
            />

            <!-- Snackbar -->
            <SnackbarManager :snackbar="currentSnackbar" />
        </PageLayout>
    </div>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
    import { useRoute } from 'vue-router'
    import { useAuthStore } from '@/stores/auth'
    import { typedHttpClient, ValidationError } from '@/api/typed-client'
    import { useTranslations } from '@/composables/useTranslations'
    import { useSnackbar } from '@/composables/useSnackbar'
    import { useErrorHandler } from '@/composables/useErrorHandler'
    import type {
        DynamicEntity,
        EntityDefinition,
        CreateEntityRequest,
        UpdateEntityRequest,
        TreeNode,
    } from '@/types/schemas'
    import EntityTree from '@/components/entities/EntityTree.vue'
    import EntityDetails from '@/components/entities/EntityDetails.vue'
    import EntityCreateDialog from '@/components/entities/EntityCreateDialog.vue'
    import EntityEditDialog from '@/components/entities/EntityEditDialog.vue'
    import DialogManager from '@/components/common/DialogManager.vue'
    import SnackbarManager from '@/components/common/SnackbarManager.vue'
    import PageLayout from '@/components/layouts/PageLayout.vue'
    import SmartIcon from '@/components/common/SmartIcon.vue'

    const authStore = useAuthStore()
    const route = useRoute()
    const { t } = useTranslations()
    const { currentSnackbar, showSuccess } = useSnackbar()
    const { handleError } = useErrorHandler()

    // Permission check for create button
    const canCreateEntity = computed(() => {
        return (
            authStore.hasPermission('Entities', 'Create') ||
            authStore.hasPermission('Entities', 'Admin')
        )
    })

    // Reactive state
    const loading = ref(false)
    const entities = ref<DynamicEntity[]>([])
    const entityDefinitions = ref<EntityDefinition[]>([])
    const selectedEntity = ref<DynamicEntity | null>(null)
    const selectedItems = ref<string[]>([])
    const expandedItems = ref<string[]>([])
    const treeRefreshKey = ref(0)
    const error = ref('')

    // Dialog states
    const showCreateDialog = ref(false)
    const showEditDialog = ref(false)
    const showDeleteDialog = ref(false)

    // Form states
    const creating = ref(false)
    const updating = ref(false)
    const deleting = ref(false)

    // Component lifecycle flag
    const isComponentMounted = ref(false)

    // Dialog refs
    interface CreateDialogInstance {
        setFieldErrors: (errors: Record<string, string>) => void
    }

    const createDialogRef = ref<CreateDialogInstance | null>(null)

    // EntityTree ref
    interface EntityTreeInstance {
        reloadPath: (path: string) => Promise<void>
    }

    const entityTreeRef = ref<EntityTreeInstance | null>(null)

    // Computed properties
    const selectedEntityUuid = computed((): string => {
        const uuid = selectedEntity.value?.field_data?.uuid
        return typeof uuid === 'string' ? uuid : ''
    })

    const selectedEntityDefinition = computed(() => {
        if (!selectedEntity.value) {
            return null
        }
        return (
            entityDefinitions.value.find(
                def => def.entity_type === selectedEntity.value?.entity_type
            ) ?? null
        )
    })

    const deleteDialogConfig = computed(() => ({
        title: t('entities.delete.title'),
        message: t('entities.delete.message'),
        maxWidth: '500px',
        persistent: false,
    }))

    /**
     * Get the parent directory path from an entity path.
     * If the path is the root, returns '/'.
     */
    const getParentDirectoryPath = (path: string): string => {
        if (!path || path === '/' || path === '') {
            return '/'
        }
        // Normalize path - remove trailing slash if present
        const normalizedPath = path.endsWith('/') ? path.slice(0, -1) : path
        // Get parent directory
        const parentPath = normalizedPath.split('/').slice(0, -1).join('/') || '/'
        return parentPath
    }

    // Methods
    const loadEntityDefinitions = async () => {
        if (!isComponentMounted.value || !authStore.isAuthenticated) {
            return
        }

        try {
            const response = await typedHttpClient.getEntityDefinitions()
            entityDefinitions.value = response.data || []
        } catch (err) {
            console.error('Failed to load entity definitions:', err)
            error.value = err instanceof Error ? err.message : 'Failed to load entity definitions'
        }
    }

    const loadEntities = async () => {
        if (!isComponentMounted.value || !authStore.isAuthenticated) {
            return
        }
        loading.value = true
        error.value = ''
        try {
            // Trigger tree reload via refresh key; entity list is now lazy path-based
            // Only increment if this is not the initial load
            treeRefreshKey.value++
        } finally {
            loading.value = false
        }
    }

    const updateExpandedItems = (items: string[]) => {
        expandedItems.value = items
    }

    const handleTreeSelection = (items: string[]) => {
        selectedItems.value = items
    }

    const handleItemClick = async (item: TreeNode) => {
        if (item.uuid && item.entity_type) {
            try {
                loading.value = true
                const entity = await typedHttpClient.getEntity(item.entity_type, item.uuid)
                selectedEntity.value = entity
                selectedItems.value = [item.uuid]
            } catch (err) {
                console.error('Failed to load entity details:', err)
                handleError(err)
            } finally {
                loading.value = false
            }
        }
    }

    const createEntity = async (data: CreateEntityRequest) => {
        creating.value = true

        try {
            await typedHttpClient.createEntity(data.entity_type, data)
            showCreateDialog.value = false

            // Get the path where the entity was created
            const entityPath = data.data?.path as string | undefined

            // Determine the path to reload using the same logic as deletion:
            // - If path has multiple segments (e.g., /test/entity-name), get parent directory
            // - If path has single segment (e.g., /test), it's already the directory path
            let pathToReload = '/'
            if (entityPath && entityPath !== '/') {
                const segments = entityPath.split('/').filter(s => s)
                // If path has more than one segment, it's likely a full entity path, get parent
                // Otherwise, it's likely a directory path, use it directly
                if (segments.length > 1) {
                    pathToReload = getParentDirectoryPath(entityPath)
                } else {
                    // Single segment, it's the directory path
                    pathToReload = entityPath
                }
            }

            // Reload the specific path instead of reloading the entire tree
            if (entityTreeRef.value) {
                await entityTreeRef.value.reloadPath(pathToReload)
            } else {
                // Fallback: refresh the entire tree if ref is not available
                treeRefreshKey.value += 1
            }

            showSuccess('Entity created successfully')
        } catch (err) {
            // Handle structured validation errors - keep dialog open for field errors
            if (err instanceof ValidationError) {
                // Convert violations to a field error map
                const fieldErrors: Record<string, string> = {}
                for (const violation of err.violations) {
                    fieldErrors[violation.field] = violation.message
                }

                // Set field errors on the dialog - keep dialog open
                if (createDialogRef.value) {
                    createDialogRef.value.setFieldErrors(fieldErrors)
                }

                // Error message is handled by global error handler
                handleError(err)
            } else {
                // Error message is handled by global error handler
                handleError(err)
            }
        } finally {
            creating.value = false
        }
    }

    const editEntity = () => {
        showEditDialog.value = true
    }

    const updateEntity = async (data: UpdateEntityRequest) => {
        if (!selectedEntity.value) {
            return
        }

        updating.value = true

        try {
            await typedHttpClient.updateEntity(
                selectedEntity.value.entity_type,
                selectedEntityUuid.value,
                data
            )

            // Update the selected entity
            selectedEntity.value = {
                ...selectedEntity.value,
                ...data,
            } as DynamicEntity

            // Update the list
            const index = entities.value.findIndex(
                e => e.field_data?.uuid === selectedEntityUuid.value
            )
            if (index !== -1 && selectedEntity.value) {
                entities.value[index] = selectedEntity.value
            }

            showEditDialog.value = false

            showSuccess('Entity updated successfully')
        } catch (err) {
            handleError(err)
        } finally {
            updating.value = false
        }
    }

    const deleteEntity = async () => {
        if (!selectedEntity.value) {
            return
        }

        deleting.value = true

        try {
            // Get the path of the entity before deletion
            const entityPath = selectedEntity.value.field_data?.path as string | undefined

            // The entity path might be:
            // 1. A directory path like "/test" - reload it directly
            // 2. A full entity path like "/test/entity-name" - reload parent "/test"
            // For now, if path exists and is not root, use it directly (it's likely the directory)
            // If it looks like a full path (has more than one segment), get parent
            let pathToReload = '/'
            if (entityPath && entityPath !== '/') {
                const segments = entityPath.split('/').filter(s => s)
                // If path has more than one segment, it's likely a full entity path, get parent
                // Otherwise, it's likely a directory path, use it directly
                if (segments.length > 1) {
                    pathToReload = getParentDirectoryPath(entityPath)
                } else {
                    // Single segment, it's the directory path
                    pathToReload = entityPath
                }
            }

            await typedHttpClient.deleteEntity(
                selectedEntity.value.entity_type,
                selectedEntityUuid.value
            )

            // Remove from list
            entities.value = entities.value.filter(
                e => e.field_data?.uuid !== selectedEntityUuid.value
            )
            selectedEntity.value = null
            selectedItems.value = []

            showDeleteDialog.value = false

            // Reload the path to update the tree
            if (entityTreeRef.value) {
                await entityTreeRef.value.reloadPath(pathToReload)
            }

            showSuccess(t('entities.delete.success'))
        } catch (err) {
            handleError(err)
        } finally {
            deleting.value = false
        }
    }

    // Lifecycle
    onMounted(async () => {
        if (isComponentMounted.value) {
            return
        }
        isComponentMounted.value = true
        await loadEntityDefinitions()
        // Don't call loadEntities() here - EntityTree will load via refreshKey watcher with immediate:true

        // Check for query params to open dialogs
        if (route.query.create === 'true') {
            await nextTick()
            showCreateDialog.value = true
            // Remove query param from URL
            window.history.replaceState({}, '', '/entities')
        }
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
