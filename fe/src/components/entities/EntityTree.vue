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
                @update:expanded-items="$emit('update:expandedItems', $event)"
                @item-click="handleItemClickProxy"
                @selection-change="$emit('selection-change', $event)"
            />
        </div>
    </div>
</template>

<script setup lang="ts">
    import { onMounted, ref, watch } from 'vue'
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
                    children: [],
                })
            } else {
                // Get icon from entity definition if available
                let icon = 'mdi-database' // default
                if (node.entity_type && props.entityDefinitions.length > 0) {
                    const entityDef = props.entityDefinitions.find(def => def.entity_type === node.entity_type)
                    if (entityDef?.icon) {
                        icon = entityDef.icon
                    }
                }
                
                files.push({
                    id: node.entity_uuid || node.path,
                    title: node.name,
                    icon,
                    entity_type: node.entity_type,
                    uuid: node.entity_uuid,
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

    async function handleItemClickProxy(item: TreeNode) {
        // Folder node
        if (item.id.startsWith('folder:')) {
            const folderPath = item.id.replace('folder:', '')
            await loadPath(folderPath, item)
            // Expand this folder after loading
            emit('update:expandedItems', [...new Set([...(props.expandedItems || []), item.id])])
            return
        }
        emit('item-click', item)
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
