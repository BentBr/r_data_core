import { ref, computed, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import { typedHttpClient } from '@/api/typed-client'

interface BrowseNode {
    kind: 'folder' | 'file'
    name: string
    path: string
    entity_uuid?: string | null
    entity_type?: string | null
    published: boolean
}

export default defineComponent({
    name: 'EntityPathPicker',
    components: {
        SmartIcon,
    },
    props: {
        path: { type: String, required: true },
        parentUuid: { type: String as PropType<string | null>, default: null },
        errorMessages: { type: Array as PropType<string[]>, default: () => [] },
    },
    emits: ['update:path', 'update:parentUuid'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        const pathSuggestions = ref<BrowseNode[]>([])
        const pathLoading = ref(false)
        const pathSearchTerm = ref('')
        let pathDebounceTimer: ReturnType<typeof setTimeout> | null = null

        const parentSuggestions = ref<any[]>([])
        const parentLoading = ref(false)
        const selectedParentDisplay = ref<string | null>(null)
        let parentDebounceTimer: ReturnType<typeof setTimeout> | null = null
        let pathSetByParent = false

        const pathValue = computed({
            get: () => props.path,
            set: (val: string | BrowseNode | null | undefined) => {
                let p = '/'
                if (typeof val === 'string') p = val
                else if (val && typeof val === 'object') p = val.path
                emit('update:path', p)
            }
        })

        const parentUuidValue = computed({
            get: () => props.parentUuid,
            set: (val: string | null | undefined) => emit('update:parentUuid', val ?? null)
        })

        const normalizedPath = computed(() => {
            const currentPath = (props.path || '').trim()
            if (!currentPath || currentPath === '/') return '/'
            return currentPath.endsWith('/') ? currentPath.slice(0, -1) || '/' : currentPath
        })

        const isRootPath = computed(() => normalizedPath.value === '/')

        const selectedPathNode = computed(() => {
            return pathSuggestions.value.find(node => node.path === normalizedPath.value) ?? null
        })

        const newFolderName = computed(() => {
            if (isRootPath.value) return ''
            return normalizedPath.value.split('/').filter(Boolean).pop() ?? ''
        })

        const isCreatingNewFolder = computed(() => {
            return !isRootPath.value && !selectedPathNode.value && Boolean(newFolderName.value)
        })

        const pathHint = computed(() => {
            if (isRootPath.value) return t('entities.create.root_folder_hint')
            if (isCreatingNewFolder.value) {
                return t('entities.create.new_folder_hint', { folder: newFolderName.value })
            }
            return t('entities.create.path_hint')
        })

        const onPathInput = (value: string | null) => {
            pathSearchTerm.value = value ?? ''
            if (pathDebounceTimer) clearTimeout(pathDebounceTimer)
            if (!value || value === '/' || value.length < 2) {
                pathSuggestions.value = []
                return
            }
            pathDebounceTimer = setTimeout(() => {
                void (async () => {
                    pathLoading.value = true
                    try {
                        const result = await typedHttpClient.searchEntitiesByPath(value, 10)
                        pathSuggestions.value = result.data
                    } finally {
                        pathLoading.value = false
                    }
                })()
            }, 350)
        }

        const onPathSelected = (value: any) => {
            if (pathSetByParent) return
            let node: BrowseNode | undefined
            if (value && typeof value === 'object') node = value
            else node = pathSuggestions.value.find(n => n.path === value)

            if (node?.entity_uuid) {
                pathSetByParent = true
                parentUuidValue.value = node.entity_uuid
                selectedParentDisplay.value = node.path
                setTimeout(() => { pathSetByParent = false }, 50)
            } else {
                parentUuidValue.value = null
                selectedParentDisplay.value = null
            }
        }

        const onParentDropdownClick = async () => {
            if (parentLoading.value) return
            parentLoading.value = true
            try {
                const result = await typedHttpClient.browseByPath(props.path || '/', 10)
                parentSuggestions.value = result.data.filter(n => n.kind === 'file' && n.entity_uuid)
            } finally {
                parentLoading.value = false
            }
        }

        const onParentSearch = (term: string | null) => {
            if (parentDebounceTimer) clearTimeout(parentDebounceTimer)
            if (!term) { void onParentDropdownClick(); return }
            parentDebounceTimer = setTimeout(() => {
                void (async () => {
                    parentLoading.value = true
                    try {
                        const result = await typedHttpClient.searchEntitiesByPath(term, 10)
                        parentSuggestions.value = result.data.filter(n => n.kind === 'file' && n.entity_uuid)
                    } finally {
                        parentLoading.value = false
                    }
                })()
            }, 350)
        }

        const onParentSelect = (uuid: string | null) => {
            if (pathSetByParent && uuid !== null) return
            parentUuidValue.value = uuid
            if (uuid) {
                const selected = parentSuggestions.value.find(s => s.entity_uuid === uuid || s.value === uuid)
                if (selected) {
                    pathSetByParent = true
                    pathValue.value = selected.path || selected.title
                    selectedParentDisplay.value = selected.path || selected.title
                    setTimeout(() => { pathSetByParent = false }, 50)
                }
            } else {
                selectedParentDisplay.value = null
            }
        }

        return {
            t, pathValue, parentUuidValue, pathSuggestions, pathLoading, pathSearchTerm,
            parentSuggestions, parentLoading, selectedParentDisplay, pathHint,
            isRootPath, isCreatingNewFolder, newFolderName,
            onPathInput, onPathSelected, onParentDropdownClick, onParentSearch, onParentSelect,
        }
    }
})
