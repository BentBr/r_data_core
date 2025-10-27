<template>
    <div class="entity-tree">
        <div class="d-flex justify-space-between align-center mb-4">
            <h3 class="text-h6">{{ t('entities.tree.title') }}</h3>
            <div class="d-flex gap-1">
                <v-btn
                    size="small"
                    variant="text"
                    :disabled="loading"
                    @click="expandAll"
                >
                    {{ t('entities.tree.expand_all') }}
                </v-btn>
                <v-btn
                    size="small"
                    variant="text"
                    :disabled="loading"
                    @click="collapseAll"
                >
                    {{ t('entities.tree.collapse_all') }}
                </v-btn>
            </div>
        </div>

        <div
            v-if="loading"
            class="d-flex justify-center align-center pa-8"
        >
            <v-progress-circular
                indeterminate
                color="primary"
            />
            <span class="ml-3">{{ t('entities.tree.loading') }}</span>
        </div>

        <div
            v-else-if="!treeItems.length"
            class="d-flex justify-center align-center pa-8"
        >
            <v-icon
                icon="mdi-database-off"
                size="large"
                color="grey"
                class="mr-3"
            />
            <span class="text-grey">{{ t('entities.tree.no_entities') }}</span>
        </div>

        <div
            v-else
            class="tree-canvas"
        >
            <TreeView
                :items="treeItems"
                :loading="loading"
                :expanded-items="expandedItems"
                @update:expanded-items="handleExpandedItemsChange"
                @item-click="handleItemClickProxy"
                @selection-change="$emit('selection-change', $event)"
            />
        </div>
    </div>
</template>

<script setup lang="ts">
    import { computed, onMounted, ref, watch } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { typedHttpClient } from '@/api/typed-client'
    import type { TreeNode, EntityDefinition } from '@/types/schemas'
    import TreeView from '@/components/common/TreeView.vue'

    interface Props {
        rootPath?: string
        loading?: boolean
        expandedItems?: string[]
        refreshKey?: number
        entityDefinitions?: EntityDefinition[]
    }

    interface Emits {
        (e: 'update:expandedItems', items: string[]): void
        (e: 'item-click', item: TreeNode): void
        (e: 'selection-change', items: string[]): void
    }

    const props = withDefaults(defineProps<Props>(), {
        rootPath: '/',
        loading: false,
        expandedItems: () => [],
        refreshKey: 0,
        entityDefinitions: () => [],
    })

    const emit = defineEmits<Emits>()

    const { t } = useTranslations()

    const treeItems = ref<TreeNode[]>([])
    const loadedPaths = ref(new Set<string>())

    function toFolderId(path: string) {
        return `folder:${path}`
    }

    function buildNodesForPath(
        path: string,
        payload: Array<{
            kind: 'folder' | 'file'
            name: string
            path: string
            entity_uuid?: string
            entity_type?: string
            has_children?: boolean
        }>
    ): TreeNode[] {
        const children: TreeNode[] = []
        const folders: TreeNode[] = []
        const files: TreeNode[] = []

        for (const node of payload) {
            if (node.kind === 'folder') {
                folders.push({
                    id: toFolderId(node.path),
                    title: node.name,
                    icon: 'mdi-folder',
                    // Only add children array if has_children is true (so arrow shows)
                    children: node.has_children ? [] : undefined,
                    path: node.path,
                })
            } else {
                // Get icon from entity definition if available
                let icon = 'mdi-database' // default
                if (node.entity_type && iconMap.value.has(node.entity_type)) {
                    icon = iconMap.value.get(node.entity_type) || icon
                }
                
                files.push({
                    id: node.entity_uuid || node.path,
                    title: node.name,
                    icon,
                    entity_type: node.entity_type,
                    uuid: node.entity_uuid,
                    path: node.path,
                    // Only add children array if has_children is true (so arrow shows)
                    children: node.has_children ? [] : undefined,
                })
            }
        }

        // Folders first then files (payload already sorted from BE, but ensure order)
        children.push(...folders, ...files)
        return children
    }

    async function loadPath(path: string, attachTo?: TreeNode) {
        if (loadedPaths.value.has(path)) {
            return
        }
        const { data } = await typedHttpClient.browseByPath(path, 100, 0)
        const nodes = buildNodesForPath(path, data)
        if (attachTo) {
            attachTo.children = nodes
        } else {
            treeItems.value = nodes
        }
        loadedPaths.value.add(path)
    }

    const expandAll = () => {
        const allIds: string[] = []
        const collectIds = (items: TreeNode[]) => {
            for (const item of items) {
                allIds.push(item.id)
                if (item.children && Array.isArray(item.children)) {
                    collectIds(item.children as TreeNode[])
                }
            }
        }
        collectIds(treeItems.value)
        emit('update:expandedItems', allIds)
    }

    const collapseAll = () => {
        emit('update:expandedItems', [])
    }

    // helper placeholder for potential deep expansion logic in the future

    async function handleExpandedItemsChange(newExpandedItems: string[]) {
        // Find newly expanded items that don't have loaded children yet
        const newlyExpanded = newExpandedItems.filter(id => !props.expandedItems?.includes(id))
        
        for (const expandedId of newlyExpanded) {
            const node = findNodeById(treeItems.value, expandedId)
            if (node && Array.isArray(node.children) && node.children.length === 0) {
                await loadChildrenForNode(node)
            }
        }
        
        emit('update:expandedItems', newExpandedItems)
    }
    
    function findNodeById(items: TreeNode[], id: string): TreeNode | null {
        for (const item of items) {
            if (item.id === id) {
                return item
            }
            if (item.children && Array.isArray(item.children)) {
                const found = findNodeById(item.children as TreeNode[], id)
                if (found) {
                    return found
                }
            }
        }
        return null
    }
    
    async function handleItemClickProxy(item: TreeNode) {
        // Regular click on item (not for expansion) - expansion is handled by handleExpandedItemsChange
        emit('item-click', item)
    }
    
    async function loadChildrenForNode(item: TreeNode) {
        // Determine the path to load based on node type
        let targetPath: string
        
        if (item.id.startsWith('folder:')) {
            // It's a folder, use the folder path
            targetPath = item.id.replace('folder:', '')
        } else if (item.path) {
            // It's an entity file, load children from its path
            targetPath = item.path
        } else {
            return
        }
        
        try {
            const { data } = await typedHttpClient.browseByPath(targetPath, 100, 0)
            const nodes = buildNodesForPath(targetPath, data)
            // Update the item's children
            item.children = nodes
        } catch (error) {
            console.error('Error loading children for node:', error)
        }
    }

    // Icon lookup map - computed to create a map of entity_type -> icon
    const iconMap = computed(() => {
        const map = new Map<string, string>()
        for (const def of props.entityDefinitions) {
            if (def.icon) {
                map.set(def.entity_type, def.icon)
            }
        }
        return map
    })

    function updateIconsInTree(items: TreeNode[]) {
        for (const item of items) {
            if (item.entity_type) {
                const icon = iconMap.value.get(item.entity_type)
                if (icon) {
                    item.icon = icon
                }
            }
            if (item.children && Array.isArray(item.children)) {
                updateIconsInTree(item.children as TreeNode[])
            }
        }
    }

    // initial load and refresh handling
    // Note: loadPath is triggered by the refreshKey watcher, not onMounted
    // This prevents duplicate API calls when the parent component increments refreshKey on mount
    onMounted(() => {
        // Don't load here - wait for refreshKey to be set by parent
    })

    watch(
        () => props.rootPath,
        p => {
            loadedPaths.value.clear()
            treeItems.value = []
            loadPath(p)
        }
    )

    watch(
        () => props.refreshKey,
        () => {
            loadedPaths.value.clear()
            treeItems.value = []
            loadPath(props.rootPath)
        },
        { immediate: true }
    )

    // Watch for entityDefinitions changes and update icons in-place (no extra API calls)
    watch(
        () => props.entityDefinitions,
        () => {
            if (treeItems.value.length > 0) {
                updateIconsInTree(treeItems.value)
            }
        },
        { deep: true }
    )
</script>

<style scoped>
    .entity-tree {
        height: 100%;
        display: flex;
        flex-direction: column;
    }
    .tree-canvas {
        min-height: 500px;
        border: 1px solid rgba(0, 0, 0, 0.12);
        border-radius: 6px;
        padding: 8px;
    }
</style>
