<template>
    <v-treeview
        v-model:opened="expandedItems"
        :items="items"
        :loading="loading"
        item-title="title"
        item-value="id"
        item-children="children"
        activatable
        hoverable
        :open-on-click="false"
        :expand-on-click="false"
        @update:active="handleSelection"
    >
        <template #prepend="{ item }">
            <v-icon
                :icon="item.icon || 'mdi-file-document'"
                :color="getItemColor(item)"
                size="small"
            />
        </template>
        <template #title="{ item }">
            <div
                class="d-flex align-center justify-space-between w-100 cursor-pointer"
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

    const getItemColor = (item: TreeNode) => {
        if (item.published !== undefined) {
            return item.published ? 'success' : 'grey'
        }
        return item.color || 'primary'
    }
</script>
