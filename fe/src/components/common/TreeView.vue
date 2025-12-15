<template>
    <v-treeview
        v-model:opened="expandedItems"
        :items="items"
        :loading="loading"
        item-title="title"
        item-value="id"
        item-children="children"
        expand-icon=""
        collapse-icon=""
        activatable
        hoverable
        :open-on-click="false"
        :expand-on-click="false"
        @update:active="handleSelection"
    >
        <template #prepend="{ item }">
            <div class="d-flex align-center tree-prepend">
                <div
                    v-if="hasChildren(item)"
                    class="tree-toggle"
                    @click.stop="toggleItem(item)"
                >
                    <SmartIcon
                        :icon="expandedItems.includes(item.id) ? 'chevron-down' : 'chevron-right'"
                        :size="16"
                        class="toggle-icon"
                    />
                </div>
                <div
                    v-else
                    class="tree-toggle-spacer"
                />
                <div
                    class="tree-icon-wrapper mr-2"
                    :class="{ 'tree-icon-disabled': isUnpublished(item) }"
                >
                    <SmartIcon
                        :icon="item.icon ?? 'file-text'"
                        :size="20"
                        class="tree-icon"
                    />
                </div>
            </div>
        </template>
        <template #title="{ item }">
            <div
                class="d-flex align-center justify-space-between w-100 cursor-pointer tree-row"
                :class="{
                    'tree-disabled': isUnpublished(item) || item.disabled,
                    'tree-strikethrough': isUnpublished(item),
                }"
                @click="handleItemClick(item)"
            >
                <span>{{ item.title }}</span>
                <span
                    v-if="item.entity_type && item.entity_type !== 'group'"
                    class="text-caption text-grey"
                >
                    {{ item.entity_type }}
                </span>
            </div>
        </template>
    </v-treeview>
</template>

<script setup lang="ts">
    import { ref, watch } from 'vue'
    import SmartIcon from './SmartIcon.vue'
    import type { TreeNode } from '@/types/schemas'

    interface Props {
        items: TreeNode[]
        loading?: boolean
        expandedItems?: string[]
    }

    interface Emits {
        (e: 'update:expandedItems', items: string[]): void
        (e: 'item-click', item: TreeNode): void
        (e: 'selection-change', items: string[]): void
    }

    const props = withDefaults(defineProps<Props>(), {
        loading: false,
        expandedItems: () => [],
    })

    const emit = defineEmits<Emits>()

    const expandedItems = ref<string[]>(props.expandedItems)
    const syncingFromProps = ref(false)

    const isUnpublished = (item: TreeNode) => {
        if (!item.entity_type || item.entity_type === 'group') {
            return false
        }
        const val = item.published as unknown
        return val === false || val === null || val === 0 || val === 'false'
    }

    // Keep local opened state in sync with parent-provided expandedItems
    watch(
        () => props.expandedItems,
        v => {
            syncingFromProps.value = true
            expandedItems.value = v
            // microtask flag reset to avoid re-emit loop
            queueMicrotask(() => {
                syncingFromProps.value = false
            })
        }
    )

    watch(expandedItems, newValue => {
        emit('update:expandedItems', newValue)
    })

    const handleSelection = (items: string[]) => {
        emit('selection-change', items)
    }

    const handleItemClick = (item: TreeNode) => {
        emit('item-click', item)
    }

    const hasChildren = (item: TreeNode): boolean => {
        return item.children !== undefined && Array.isArray(item.children)
    }

    const toggleItem = (item: TreeNode) => {
        const index = expandedItems.value.indexOf(item.id)
        if (index > -1) {
            expandedItems.value = expandedItems.value.filter(id => id !== item.id)
        } else {
            expandedItems.value = [...expandedItems.value, item.id]
        }
    }
</script>

<style scoped>
    /* Tree prepend container */
    .tree-prepend {
        display: flex;
        align-items: center;
        gap: 4px;
    }

    /* Toggle icon for expand/collapse */
    .tree-toggle {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 24px;
        height: 24px;
        cursor: pointer;
        flex-shrink: 0;
    }

    .tree-toggle-spacer {
        width: 24px;
        flex-shrink: 0;
    }

    .toggle-icon {
        opacity: 1 !important;
        visibility: visible !important;
    }

    /* Ensure icons are always visible, even when items are active */
    .tree-icon-wrapper {
        display: inline-flex;
        align-items: center;
    }

    :deep(.v-treeview-item) .tree-icon,
    :deep(.v-treeview-item) .tree-icon-wrapper {
        opacity: 1 !important;
        display: inline-flex !important;
        visibility: visible !important;
    }

    .tree-row.tree-disabled {
        opacity: 0.55;
        color: rgba(var(--v-theme-on-surface), 0.6);
    }

    .tree-row.tree-strikethrough {
        text-decoration: line-through;
    }

    .tree-icon-disabled {
        opacity: 0.55;
        color: rgba(var(--v-theme-on-surface), 0.6);
    }

    :deep(.v-treeview-item--active) .tree-icon,
    :deep(.v-treeview-item--active) .tree-icon-wrapper {
        opacity: 1 !important;
        display: inline-flex !important;
        visibility: visible !important;
    }

    /* Ensure prepend slot is visible and has proper layout */
    :deep(.v-treeview-item__prepend) {
        display: flex !important;
        align-items: center !important;
        gap: 8px !important;
        min-width: auto !important;
    }

    /* Hide Vuetify's internal expand/collapse icons - we use our own in prepend slot */
    :deep(.v-treeview-item__toggle) {
        display: none !important;
        width: 0 !important;
        height: 0 !important;
        overflow: hidden !important;
        visibility: hidden !important;
    }

    /* Also hide any icon inside the toggle */
    :deep(.v-treeview-item__toggle .v-icon) {
        display: none !important;
    }

    /* Hide the spacer that Vuetify adds for items without children */
    :deep(.v-treeview-item__level) {
        width: 0 !important;
    }
</style>
