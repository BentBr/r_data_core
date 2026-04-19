import { computed, ref, watch, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { typedHttpClient } from '@/api/typed-client'
import type { TreeNode, EntityDefinition } from '@/types/schemas'
import TreeView from '@/shared/components/TreeView/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'

export default defineComponent({
    name: 'EntityTree',
    components: {
        TreeView,
        SmartIcon,
    },
    props: {
        rootPath: {
            type: String,
            default: '/',
        },
        loading: {
            type: Boolean,
            default: false,
        },
        expandedItems: {
            type: Array as PropType<string[]>,
            default: () => [],
        },
        refreshKey: {
            type: Number,
            default: 0,
        },
        entityDefinitions: {
            type: Array as PropType<EntityDefinition[]>,
            default: () => [],
        },
    },
    emits: ['update:expandedItems', 'item-click', 'selection-change'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        const treeItems = ref<TreeNode[]>([])
        const loadedPaths = ref(new Set<string>())

        const iconMap = computed(() => {
            const map = new Map<string, string>()
            for (const def of props.entityDefinitions) {
                if (def.icon) map.set(def.entity_type, def.icon)
            }
            return map
        })

        function toFolderId(path: string) { return `folder:${path}` }

        function buildNodesForPath(_path: string, payload: any[]): TreeNode[] {
            const children: TreeNode[] = []
            const folders: TreeNode[] = []
            const files: TreeNode[] = []
            for (const node of payload) {
                if (node.kind === 'folder') {
                    folders.push({
                        id: toFolderId(node.path), title: node.name, icon: 'folder',
                        children: node.has_children ? [] : undefined, path: node.path,
                    })
                } else {
                    let icon = 'database'
                    if (node.entity_type && iconMap.value.has(node.entity_type)) {
                        icon = iconMap.value.get(node.entity_type) ?? icon
                    }
                    files.push({
                        id: node.entity_uuid ?? node.path, title: node.name, icon,
                        entity_type: node.entity_type ?? undefined, uuid: node.entity_uuid ?? undefined,
                        path: node.path, published: node.published,
                        children: node.has_children ? [] : undefined,
                    })
                }
            }
            children.push(...folders, ...files)
            return children
        }

        async function loadPath(path: string, attachTo?: TreeNode) {
            if (loadedPaths.value.has(path)) return
            const { data } = await typedHttpClient.browseByPath(path, 100, 0)
            const nodes = buildNodesForPath(path, data)
            updateNodesFromDefinitions(nodes)
            if (attachTo) attachTo.children = nodes
            else treeItems.value = nodes
            loadedPaths.value.add(path)
        }

        async function reloadPath(path: string) {
            const normalizedPath = path.endsWith('/') && path !== '/' ? path.slice(0, -1) : path
            if (normalizedPath === '/' || normalizedPath === '') {
                const currentExpandedItems = [...props.expandedItems]
                loadedPaths.value.delete('/')
                await loadPath('/')
                emit('update:expandedItems', currentExpandedItems)
                return
            }
            const node = findNodeByPath(treeItems.value, normalizedPath)
            if (node) {
                const wasExpanded = props.expandedItems.includes(node.id)
                loadedPaths.value.delete(normalizedPath)
                await loadChildrenForNode(node)
                if (wasExpanded) {
                    const newExpandedItems = [...props.expandedItems]
                    if (!newExpandedItems.includes(node.id)) newExpandedItems.push(node.id)
                    emit('update:expandedItems', newExpandedItems)
                }
            } else {
                const parentPath = normalizedPath.split('/').slice(0, -1).join('/') || '/'
                if (parentPath !== '/' && parentPath !== '') await reloadPath(parentPath)
                else {
                    const currentExpandedItems = [...props.expandedItems]
                    loadedPaths.value.delete('/')
                    await loadPath('/')
                    emit('update:expandedItems', currentExpandedItems)
                }
            }
        }

        function findNodeByPath(items: TreeNode[], targetPath: string): TreeNode | null {
            for (const item of items) {
                if ((item.id.startsWith('folder:') && item.path === targetPath) || item.path === targetPath) return item
                if (item.children && Array.isArray(item.children)) {
                    const found = findNodeByPath(item.children as TreeNode[], targetPath)
                    if (found) return found
                }
            }
            return null
        }

        const expandAll = () => {
            const allIds: string[] = []
            const collectIds = (items: TreeNode[]) => {
                for (const item of items) {
                    allIds.push(item.id)
                    if (item.children && Array.isArray(item.children)) collectIds(item.children as TreeNode[])
                }
            }
            collectIds(treeItems.value)
            emit('update:expandedItems', allIds)
        }

        const collapseAll = () => emit('update:expandedItems', [])

        async function handleExpandedItemsChange(newExpandedItems: string[]) {
            const newlyExpanded = newExpandedItems.filter(id => !props.expandedItems.includes(id))
            for (const expandedId of newlyExpanded) {
                const node = findNodeById(treeItems.value, expandedId)
                if (node && Array.isArray(node.children) && node.children.length === 0) await loadChildrenForNode(node)
            }
            emit('update:expandedItems', newExpandedItems)
        }

        function findNodeById(items: TreeNode[], id: string): TreeNode | null {
            for (const item of items) {
                if (item.id === id) return item
                if (item.children && Array.isArray(item.children)) {
                    const found = findNodeById(item.children as TreeNode[], id)
                    if (found) return found
                }
            }
            return null
        }

        async function handleItemClickProxy(item: TreeNode) { emit('item-click', item) }

        async function loadChildrenForNode(item: TreeNode) {
            let targetPath: string
            if (item.id.startsWith('folder:')) targetPath = item.id.replace('folder:', '')
            else if (item.path) targetPath = item.path
            else return
            try {
                const { data } = await typedHttpClient.browseByPath(targetPath, 100, 0)
                const nodes = buildNodesForPath(targetPath, data)
                updateNodesFromDefinitions(nodes)
                item.children = nodes
            } catch (error) {
                console.error('Error loading children for node:', error)
            }
        }

        function updateNodesFromDefinitions(items: TreeNode[]) {
            for (const item of items) {
                if (item.entity_type) {
                    const icon = iconMap.value.get(item.entity_type)
                    if (icon) item.icon = icon
                }
                if (item.children && Array.isArray(item.children)) updateNodesFromDefinitions(item.children as TreeNode[])
            }
        }

        watch(() => props.rootPath, p => {
            loadedPaths.value.clear(); treeItems.value = []; void loadPath(p)
        })

        watch(() => props.refreshKey, () => {
            loadedPaths.value.clear(); treeItems.value = []; void loadPath(props.rootPath)
        }, { immediate: true })

        watch(() => props.entityDefinitions, () => {
            if (treeItems.value.length > 0) updateNodesFromDefinitions(treeItems.value)
        }, { deep: true })

        return {
            t,
            treeItems,
            expandAll,
            collapseAll,
            handleExpandedItemsChange,
            handleItemClickProxy,
            reloadPath,
            emit,
        }
    },
})
