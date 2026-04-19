import { ref, computed, onMounted, onUnmounted, nextTick, defineComponent } from 'vue'
import { useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { typedHttpClient, ValidationError } from '@/api/typed-client'
import { useTranslations } from '@/shared/composables/useTranslations'
import { useSnackbar } from '@/shared/composables/useSnackbar'
import { useErrorHandler } from '@/shared/composables/useErrorHandler'
import type {
    DynamicEntity,
    EntityDefinition,
    CreateEntityRequest,
    UpdateEntityRequest,
    TreeNode,
} from '@/types/schemas'
import EntityTree from '@/modules/entities/components/EntityTree/index.vue'
import EntityDetails from '@/modules/entities/components/EntityDetails/index.vue'
import EntityCreateDialog from '@/modules/entities/components/EntityCreateDialog/index.vue'
import EntityEditDialog from '@/modules/entities/components/EntityEditDialog/index.vue'
import DialogManager from '@/shared/components/DialogManager/index.vue'
import SnackbarManager from '@/shared/components/SnackbarManager/index.vue'
import PageLayout from '@/shared/components/PageLayout/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'

export default defineComponent({
    name: 'EntitiesPage',
    components: {
        EntityTree,
        EntityDetails,
        EntityCreateDialog,
        EntityEditDialog,
        DialogManager,
        SnackbarManager,
        PageLayout,
        SmartIcon,
    },
    setup() {
        const authStore = useAuthStore()
        const route = useRoute()
        const { t } = useTranslations()
        const { currentSnackbar, showSuccess } = useSnackbar()
        const { handleError } = useErrorHandler()

        const canCreateEntity = computed(() => {
            return (
                authStore.hasPermission('Entities', 'Create') ||
                authStore.hasPermission('Entities', 'Admin')
            )
        })

        const loading = ref(false)
        const entities = ref<DynamicEntity[]>([])
        const entityDefinitions = ref<EntityDefinition[]>([])
        const selectedEntity = ref<DynamicEntity | null>(null)
        const selectedItems = ref<string[]>([])
        const expandedItems = ref<string[]>([])
        const treeRefreshKey = ref(0)
        const error = ref('')

        const showCreateDialog = ref(false)
        const showEditDialog = ref(false)
        const showDeleteDialog = ref(false)

        const creating = ref(false)
        const updating = ref(false)
        const deleting = ref(false)

        const isComponentMounted = ref(false)

        interface DialogInstance {
            setFieldErrors: (errors: Record<string, string>) => void
        }
        const createDialogRef = ref<DialogInstance | null>(null)
        const editDialogRef = ref<DialogInstance | null>(null)

        interface EntityTreeInstance {
            reloadPath: (path: string) => Promise<void>
        }
        const entityTreeRef = ref<EntityTreeInstance | null>(null)

        const selectedEntityUuid = computed((): string => {
            const uuid = selectedEntity.value?.field_data.uuid
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

        const getParentDirectoryPath = (path: string): string => {
            if (!path || path === '/' || path === '') {
                return '/'
            }
            const normalizedPath = path.endsWith('/') ? path.slice(0, -1) : path
            const parentPath = normalizedPath.split('/').slice(0, -1).join('/') || '/'
            return parentPath
        }

        const loadEntityDefinitions = async () => {
            if (!isComponentMounted.value || !authStore.isAuthenticated) {
                return
            }
            try {
                const response = await typedHttpClient.getEntityDefinitions()
                entityDefinitions.value = response.data
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
                    const entity = await typedHttpClient.getEntity(item.entity_type, item.uuid, {
                        includeChildrenCount: true,
                    })
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
                const entityPath = data.data.path as string | undefined
                let pathToReload = '/'
                if (entityPath && entityPath !== '/') {
                    const segments = entityPath.split('/').filter(s => s)
                    if (segments.length > 1) {
                        pathToReload = getParentDirectoryPath(entityPath)
                    } else {
                        pathToReload = entityPath
                    }
                }
                if (entityTreeRef.value) {
                    await entityTreeRef.value.reloadPath(pathToReload)
                } else {
                    treeRefreshKey.value += 1
                }
                showSuccess('Entity created successfully')
            } catch (err) {
                if (err instanceof ValidationError) {
                    const fieldErrors: Record<string, string> = {}
                    for (const violation of err.violations) {
                        fieldErrors[violation.field] = violation.message
                    }
                    if (createDialogRef.value) {
                        createDialogRef.value.setFieldErrors(fieldErrors)
                    }
                    handleError(err)
                } else {
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
                selectedEntity.value = {
                    ...selectedEntity.value,
                    ...data,
                } as DynamicEntity
                const index = entities.value.findIndex(
                    e => e.field_data.uuid === selectedEntityUuid.value
                )
                if (index !== -1) {
                    entities.value[index] = selectedEntity.value
                }
                showEditDialog.value = false
                showSuccess('Entity updated successfully')
            } catch (err) {
                if (err instanceof ValidationError) {
                    const fieldErrors: Record<string, string> = {}
                    for (const violation of err.violations) {
                        fieldErrors[violation.field] = violation.message
                    }
                    if (editDialogRef.value) {
                        editDialogRef.value.setFieldErrors(fieldErrors)
                    }
                    handleError(err)
                } else {
                    handleError(err)
                }
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
                const entityPath = selectedEntity.value.field_data.path as string | undefined
                let pathToReload = '/'
                if (entityPath && entityPath !== '/') {
                    const segments = entityPath.split('/').filter(s => s)
                    if (segments.length > 1) {
                        pathToReload = getParentDirectoryPath(entityPath)
                    } else {
                        pathToReload = entityPath
                    }
                }
                await typedHttpClient.deleteEntity(
                    selectedEntity.value.entity_type,
                    selectedEntityUuid.value
                )
                entities.value = entities.value.filter(
                    e => e.field_data.uuid !== selectedEntityUuid.value
                )
                selectedEntity.value = null
                selectedItems.value = []
                showDeleteDialog.value = false
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

        onMounted(async () => {
            if (isComponentMounted.value) {
                return
            }
            isComponentMounted.value = true
            await loadEntityDefinitions()
            if (route.query.create === 'true') {
                await nextTick()
                showCreateDialog.value = true
                window.history.replaceState({}, '', '/entities')
            }
        })

        onUnmounted(() => {
            isComponentMounted.value = false
        })

        return {
            t,
            loading,
            canCreateEntity,
            entities,
            entityDefinitions,
            selectedEntity,
            selectedItems,
            expandedItems,
            treeRefreshKey,
            error,
            showCreateDialog,
            showEditDialog,
            showDeleteDialog,
            creating,
            updating,
            deleting,
            createDialogRef,
            editDialogRef,
            entityTreeRef,
            selectedEntityDefinition,
            deleteDialogConfig,
            currentSnackbar,
            loadEntities,
            updateExpandedItems,
            handleTreeSelection,
            handleItemClick,
            createEntity,
            editEntity,
            updateEntity,
            deleteEntity,
        }
    },
})
