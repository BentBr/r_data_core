import { ref, computed, watch, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import { getDialogMaxWidth, buttonConfigs } from '@/design-system/components'
import type { EntityDefinition, CreateEntityRequest, DynamicEntity } from '@/types/schemas'
import { useFieldRendering } from '@/shared/composables/useFieldRendering'
import { typedHttpClient } from '@/api/typed-client'
import EntityPathPicker from '../EntityPathPicker/index.vue'

export default defineComponent({
    name: 'EntityCreateDialog',
    components: {
        SmartIcon,
        EntityPathPicker,
    },
    props: {
        modelValue: { type: Boolean, required: true },
        entityDefinitions: { type: Array as PropType<EntityDefinition[]>, required: true },
        loading: { type: Boolean, default: false },
        defaultParent: { type: Object as PropType<DynamicEntity | null>, default: null },
    },
    emits: ['update:modelValue', 'create'],
    setup(props, { emit }) {
        const { t } = useTranslations()
        const { getFieldComponent, getFieldRules, parseJsonFieldValue } = useFieldRendering()

        const form = ref()
        const pathPickerRef = ref<any>(null)
        const isValid = ref(false)
        const pathSuggestions = ref<any[]>([])
        const parentSuggestions = ref<any[]>([])
        const pathLoading = ref(false)
        const parentLoading = ref(false)
        const selectedParentDisplay = ref<string | null>(null)
        let pathDebounceTimer: ReturnType<typeof setTimeout> | null = null
        let parentDebounceTimer: ReturnType<typeof setTimeout> | null = null
        let pathSetByParent = false
        const formData = ref<CreateEntityRequest>({
            entity_type: '',
            data: { entity_key: '', path: '/', published: false },
            parent_uuid: null,
        })
        const fieldErrors = ref<Record<string, string>>({})

        const dialogVisible = computed({
            get: () => props.modelValue,
            set: value => emit('update:modelValue', value),
        })

        const availableEntityTypes = computed(() => {
            return props.entityDefinitions
                .filter(def => def.published)
                .map(def => ({ entity_type: def.entity_type, display_name: def.display_name }))
        })

        const selectedEntityDefinition = computed(() => {
            return props.entityDefinitions.find(def => def.entity_type === formData.value.entity_type)
        })

        const onEntityTypeChange = () => {
            formData.value.data = { entity_key: '', path: '/', published: false }
            formData.value.parent_uuid = null
            if (selectedEntityDefinition.value) {
                for (const field of selectedEntityDefinition.value.fields) {
                    if (field.ui_settings?.default !== undefined) {
                        formData.value.data[field.name] = field.ui_settings.default
                    }
                }
            }
        }

        const createEntity = async () => {
            if (!form.value?.validate()) return
            fieldErrors.value = {}
            const processedData: Record<string, unknown> = { ...formData.value.data }
            if (selectedEntityDefinition.value) {
                for (const field of selectedEntityDefinition.value.fields) {
                    const fieldName = field.name
                    if (processedData[fieldName] !== undefined) {
                        const { parsed, error } = parseJsonFieldValue(processedData[fieldName], field.field_type)
                        if (error) fieldErrors.value[fieldName] = error
                        else processedData[fieldName] = parsed
                    }
                }
            }
            if (Object.keys(fieldErrors.value).length > 0) return
            emit('create', { ...formData.value, data: processedData })
        }

        const setFieldErrors = (errors: Record<string, string>) => { fieldErrors.value = errors }

        const normalizedPath = computed(() => {
            const currentPath = String(formData.value.data.path || '').trim()
            if (!currentPath || currentPath == '/') return '/'
            return currentPath.endsWith('/') ? currentPath.slice(0, -1) || '/' : currentPath
        })

        const isRootPath = computed(() => normalizedPath.value === '/')

        const newFolderName = computed(() => {
            if (isRootPath.value) return ''
            return normalizedPath.value.split('/').filter(Boolean).pop() ?? ''
        })

        const isCreatingNewFolder = computed(() => {
            return !isRootPath.value && !pathSuggestions.value.find(node => node.path === normalizedPath.value) && Boolean(newFolderName.value)
        })

        const pathHint = computed(() => {
            if (isRootPath.value) return t('entities.create.root_folder_hint')
            if (isCreatingNewFolder.value) {
                return t('entities.create.new_folder_hint', { folder: newFolderName.value })
            }
            return t('entities.create.path_hint')
        })

        const syncPathPickerState = () => {
            if (!pathPickerRef.value) return
            pathPickerRef.value.pathSuggestions = pathSuggestions.value
            pathPickerRef.value.parentSuggestions = parentSuggestions.value
            pathPickerRef.value.pathLoading = pathLoading.value
            pathPickerRef.value.parentLoading = parentLoading.value
            pathPickerRef.value.selectedParentDisplay = selectedParentDisplay.value
        }

        const onPathInput = (value: string | null) => {
            if (pathDebounceTimer) clearTimeout(pathDebounceTimer)
            if (!value || value === '/' || value.length < 2) {
                pathSuggestions.value = []
                syncPathPickerState()
                return
            }
            pathDebounceTimer = setTimeout(() => {
                void (async () => {
                    pathLoading.value = true
                    syncPathPickerState()
                    try {
                        const result = await typedHttpClient.searchEntitiesByPath(value, 10)
                        pathSuggestions.value = result.data
                    } finally {
                        pathLoading.value = false
                        syncPathPickerState()
                    }
                })()
            }, 350)
        }

        const onPathSelected = (value: unknown) => {
            if (pathSetByParent) return
            let node: any = null
            if (value && typeof value === 'object') node = value
            else node = pathSuggestions.value.find(n => n.path === value)

            if (node?.entity_uuid) {
                pathSetByParent = true
                formData.value.parent_uuid = node.entity_uuid
                selectedParentDisplay.value = node.path
                setTimeout(() => { pathSetByParent = false }, 50)
            } else {
                formData.value.parent_uuid = null
                selectedParentDisplay.value = null
            }
            syncPathPickerState()
        }

        const onParentDropdownClick = async () => {
            if (parentLoading.value) return
            parentLoading.value = true
            syncPathPickerState()
            try {
                const result = await typedHttpClient.browseByPath(String(formData.value.data.path || '/'), 10)
                parentSuggestions.value = result.data.filter((n: any) => n.kind === 'file' && n.entity_uuid)
            } catch {
                parentSuggestions.value = []
            } finally {
                parentLoading.value = false
                syncPathPickerState()
            }
        }

        const onParentSearch = (term: string | null) => {
            if (parentDebounceTimer) clearTimeout(parentDebounceTimer)
            if (!term) {
                void onParentDropdownClick()
                return
            }
            parentDebounceTimer = setTimeout(() => {
                void (async () => {
                    parentLoading.value = true
                    syncPathPickerState()
                    try {
                        const result = await typedHttpClient.searchEntitiesByPath(term, 10)
                        parentSuggestions.value = result.data.filter((n: any) => n.kind === 'file' && n.entity_uuid)
                    } finally {
                        parentLoading.value = false
                        syncPathPickerState()
                    }
                })()
            }, 350)
        }

        const onParentSelect = (uuid: string | null) => {
            if (pathSetByParent && uuid !== null) return
            formData.value.parent_uuid = uuid
            if (uuid) {
                const selected = parentSuggestions.value.find((s: any) => s.entity_uuid === uuid || s.value === uuid)
                if (selected) {
                    pathSetByParent = true
                    formData.value.data.path = selected.path || selected.title
                    selectedParentDisplay.value = selected.path || selected.title
                    setTimeout(() => { pathSetByParent = false }, 50)
                }
            } else {
                selectedParentDisplay.value = null
            }
            syncPathPickerState()
        }

        watch(dialogVisible, visible => {
            if (!visible) {
                formData.value = { entity_type: '', data: { entity_key: '', path: '/', published: false }, parent_uuid: null }
                fieldErrors.value = {}
            }
        })

        watch(() => props.defaultParent, (newParent) => {
            if (newParent && dialogVisible.value && formData.value.entity_type) {
                formData.value.parent_uuid = (newParent.field_data.uuid as string) || null
                const parentPath = newParent.field_data.path as string
                const parentKey = newParent.field_data.entity_key as string
                formData.value.data.path = parentPath.endsWith('/') ? `${parentPath}${parentKey}` : `${parentPath}/${parentKey}`
            }
        })

        return {
            t, getFieldComponent, getFieldRules, form, pathPickerRef, isValid, formData, fieldErrors,
            dialogVisible, availableEntityTypes, selectedEntityDefinition, buttonConfigs,
            pathSuggestions, parentSuggestions, pathLoading, parentLoading, selectedParentDisplay,
            pathHint, isRootPath, isCreatingNewFolder, newFolderName,
            onEntityTypeChange, createEntity, setFieldErrors, getDialogMaxWidth,
            onPathInput, onPathSelected, onParentDropdownClick, onParentSearch, onParentSelect,
        }
    },
})
