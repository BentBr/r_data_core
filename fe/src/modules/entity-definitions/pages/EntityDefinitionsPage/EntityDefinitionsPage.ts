import { ref, computed, onMounted, onUnmounted, nextTick, defineComponent } from 'vue'
import { useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { typedHttpClient } from '@/api/typed-client'
import { useTranslations } from '@/shared/composables/useTranslations'
import { useSnackbar } from '@/shared/composables/useSnackbar'
import { useErrorHandler } from '@/shared/composables/useErrorHandler'
import type {
    EntityDefinition,
    CreateEntityDefinitionRequest,
    UpdateEntityDefinitionRequest,
    FieldDefinition,
    TreeNode,
} from '@/types/schemas'
import EntityDefinitionTree from '@/modules/entity-definitions/components/EntityDefinitionTree/index.vue'
import EntityDefinitionDetails from '@/modules/entity-definitions/components/EntityDefinitionDetails/index.vue'
import EntityDefinitionCreateDialog from '@/modules/entity-definitions/components/EntityDefinitionCreateDialog/index.vue'
import EntityDefinitionEditDialog from '@/modules/entity-definitions/components/EntityDefinitionEditDialog/index.vue'
import FieldEditor from '@/shared/forms/FieldEditor/index.vue'
import DialogManager from '@/shared/components/DialogManager/index.vue'
import SnackbarManager from '@/shared/components/SnackbarManager/index.vue'
import PageLayout from '@/shared/components/PageLayout/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'

export default defineComponent({
    name: 'EntityDefinitionsPage',
    components: {
        EntityDefinitionTree,
        EntityDefinitionDetails,
        EntityDefinitionCreateDialog,
        EntityDefinitionEditDialog,
        FieldEditor,
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

        const canCreateEntityDefinition = computed(() => {
            return (
                authStore.hasPermission('EntityDefinitions', 'Create') ||
                authStore.hasPermission('EntityDefinitions', 'Admin')
            )
        })

        const loading = ref(false)
        const entityDefinitions = ref<EntityDefinition[]>([])
        const selectedDefinition = ref<EntityDefinition | null>(null)
        const selectedItems = ref<string[]>([])
        const expandedGroups = ref<string[]>([])
        const originalDefinition = ref<EntityDefinition | null>(null)
        const error = ref('')

        const showCreateDialog = ref(false)
        const showEditDialog = ref(false)
        const showDeleteDialog = ref(false)
        const showFieldEditor = ref(false)
        const editingField = ref<FieldDefinition | undefined>(undefined)

        const creating = ref(false)
        const updating = ref(false)
        const deleting = ref(false)
        const savingChanges = ref(false)

        const isComponentMounted = ref(false)

        const hasUnsavedChanges = computed(() => {
            if (!selectedDefinition.value || !originalDefinition.value) {
                return false
            }
            if (selectedDefinition.value.fields.length !== originalDefinition.value.fields.length) {
                return true
            }
            const currentFieldsMap = new Map<string, FieldDefinition>(
                selectedDefinition.value.fields.map(field => [field.name, field])
            )
            const originalFieldsMap = new Map<string, FieldDefinition>(
                originalDefinition.value.fields.map(field => [field.name, field])
            )
            const currentFieldNames = Array.from(currentFieldsMap.keys())
            const originalFieldNames = Array.from(originalFieldsMap.keys())
            for (const fieldName of currentFieldNames) {
                if (!originalFieldsMap.has(fieldName)) return true
            }
            for (const fieldName of originalFieldNames) {
                if (!currentFieldsMap.has(fieldName)) return true
            }
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
                    currentField.unique !== originalField.unique ||
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

        const sanitizeFields = (fields: FieldDefinition[], deepCopy = false) => {
            return fields.map(field => ({
                ...field,
                constraints:
                    deepCopy && field.constraints
                        ? JSON.parse(JSON.stringify(field.constraints))
                        : field.constraints,
                ui_settings:
                    deepCopy && field.ui_settings
                        ? JSON.parse(JSON.stringify(field.ui_settings))
                        : field.ui_settings,
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
                entityDefinitions.value = response.data.map(definition => ({
                    ...definition,
                    fields: sanitizeFields(definition.fields),
                }))
            } catch (err) {
                console.error('Failed to load entity definitions:', err)
                error.value = err instanceof Error ? err.message : 'Failed to load entity definitions'
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
                if (selectedId.startsWith('group-')) {
                    if (expandedGroups.value.includes(selectedId)) {
                        expandedGroups.value = expandedGroups.value.filter(id => id !== selectedId)
                    } else {
                        expandedGroups.value.push(selectedId)
                    }
                } else {
                    const definition = entityDefinitions.value.find(d => d.uuid === selectedId)
                    if (definition) {
                        selectedDefinition.value = definition
                        originalDefinition.value = {
                            ...definition,
                            fields: sanitizeFields(definition.fields, true),
                        }
                        selectedItems.value = [selectedId]
                    }
                }
            } else {
                selectedDefinition.value = null
            }
        }

        const handleItemClick = async (item: TreeNode) => {
            if (item.entity_type === 'group') {
                const groupId = item.id
                if (expandedGroups.value.includes(groupId)) {
                    expandedGroups.value = expandedGroups.value.filter(id => id !== groupId)
                } else {
                    expandedGroups.value.push(groupId)
                }
            } else if (item.uuid) {
                try {
                    loading.value = true
                    const definition = await typedHttpClient.getEntityDefinition(item.uuid)
                    const sanitizedDefinition = {
                        ...definition,
                        fields: sanitizeFields(definition.fields),
                    }
                    selectedDefinition.value = sanitizedDefinition
                    originalDefinition.value = {
                        ...sanitizedDefinition,
                        fields: sanitizeFields(sanitizedDefinition.fields, true),
                    }
                    selectedItems.value = [item.uuid]
                    const index = entityDefinitions.value.findIndex(d => d.uuid === item.uuid)
                    if (index !== -1) {
                        entityDefinitions.value[index] = sanitizedDefinition
                    }
                } catch (err) {
                    console.error('Failed to load entity definition:', err)
                    handleError(err)
                } finally {
                    loading.value = false
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
                if (selectedDefinition.value.uuid) {
                    const reloaded = await typedHttpClient.getEntityDefinition(
                        selectedDefinition.value.uuid
                    )
                    const sanitizedReloaded = {
                        ...reloaded,
                        fields: sanitizeFields(reloaded.fields),
                    }
                    selectedDefinition.value = sanitizedReloaded
                    originalDefinition.value = {
                        ...sanitizedReloaded,
                        fields: sanitizeFields(sanitizedReloaded.fields, true),
                    }
                    const index = entityDefinitions.value.findIndex(
                        d => d.uuid === selectedDefinition.value?.uuid
                    )
                    if (index !== -1) {
                        entityDefinitions.value[index] = sanitizedReloaded
                    }
                }
                showSuccess(t('entity_definitions.details.changes_saved'))
            } catch (err) {
                handleError(err)
            } finally {
                savingChanges.value = false
            }
        }

        const createEntityDefinition = async (data: CreateEntityDefinitionRequest) => {
            creating.value = true
            try {
                await typedHttpClient.createEntityDefinition(data)
                showCreateDialog.value = false
                await loadEntityDefinitions()
                showSuccess('Entity definition created successfully')
            } catch (err) {
                handleError(err)
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
                if (selectedDefinition.value.uuid) {
                    const reloaded = await typedHttpClient.getEntityDefinition(
                        selectedDefinition.value.uuid
                    )
                    const sanitizedReloaded = {
                        ...reloaded,
                        fields: sanitizeFields(reloaded.fields),
                    }
                    selectedDefinition.value = sanitizedReloaded
                    originalDefinition.value = {
                        ...sanitizedReloaded,
                        fields: sanitizeFields(sanitizedReloaded.fields, true),
                    }
                    const index = entityDefinitions.value.findIndex(
                        d => d.uuid === selectedDefinition.value?.uuid
                    )
                    if (index !== -1) {
                        entityDefinitions.value[index] = sanitizedReloaded
                    }
                }
                showEditDialog.value = false
                showSuccess('Entity definition updated successfully')
            } catch (err) {
                handleError(err)
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
                entityDefinitions.value = entityDefinitions.value.filter(
                    d => d.uuid !== selectedDefinition.value?.uuid
                )
                selectedDefinition.value = null
                selectedItems.value = []
                showDeleteDialog.value = false
                showSuccess(t('entity_definitions.delete.success'))
            } catch (err) {
                handleError(err)
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
            const sanitizedField = {
                ...field,
                constraints: field.constraints,
                ui_settings: field.ui_settings ?? {},
            }
            if (editingField.value) {
                const index = selectedDefinition.value?.fields.findIndex(
                    f => f.name === editingField.value?.name
                )
                if (index !== -1 && index !== undefined && selectedDefinition.value) {
                    selectedDefinition.value.fields[index] = sanitizedField
                }
            } else {
                if (selectedDefinition.value) {
                    selectedDefinition.value.fields.push(sanitizedField)
                }
            }
        }

        onMounted(async () => {
            isComponentMounted.value = true
            await loadEntityDefinitions()
            if (route.query.create === 'true') {
                await nextTick()
                showCreateDialog.value = true
                window.history.replaceState({}, '', '/entity-definitions')
            }
        })

        onUnmounted(() => {
            isComponentMounted.value = false
        })

        return {
            t,
            loading,
            canCreateEntityDefinition,
            entityDefinitions,
            selectedDefinition,
            selectedItems,
            expandedGroups,
            hasUnsavedChanges,
            savingChanges,
            showCreateDialog,
            showEditDialog,
            showDeleteDialog,
            showFieldEditor,
            editingField,
            creating,
            updating,
            deleting,
            deleteDialogConfig,
            currentSnackbar,
            updateExpandedGroups,
            handleTreeSelection,
            handleItemClick,
            saveChanges,
            createEntityDefinition,
            editDefinition,
            updateEntityDefinition,
            deleteEntityDefinition,
            addField,
            editField,
            removeField,
            handleFieldSave,
        }
    },
})
