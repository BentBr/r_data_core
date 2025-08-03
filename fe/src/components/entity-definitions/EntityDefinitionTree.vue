<template>
    <v-card variant="outlined">
        <v-card-title class="text-h6 pa-3">
            <v-icon
                icon="mdi-folder-tree"
                class="mr-2"
            />
            {{ t('entity_definitions.table.display_name') }}
        </v-card-title>
        <v-card-text class="pa-0">
            <!-- Skeleton loader for initial load -->
            <div v-if="loading && entityDefinitions.length === 0">
                <v-skeleton-loader
                    type="list-item-three-line"
                    class="pa-2"
                />
                <v-skeleton-loader
                    type="list-item-three-line"
                    class="pa-2"
                />
                <v-skeleton-loader
                    type="list-item-three-line"
                    class="pa-2"
                />
            </div>

            <!-- Tree view -->
            <TreeView
                v-else
                :items="treeItems"
                :loading="loading"
                :expanded-items="expandedGroups"
                @update:expanded-items="updateExpandedGroups"
                @item-click="handleItemClick"
                @selection-change="handleSelection"
            />
        </v-card-text>
    </v-card>
</template>

<script setup lang="ts">
    import { computed } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import TreeView from '@/components/common/TreeView.vue'
    import type { EntityDefinition, TreeNode } from '@/types/schemas'

    interface Props {
        entityDefinitions: EntityDefinition[]
        loading: boolean
        expandedGroups: string[]
    }

    interface Emits {
        (e: 'update:expandedGroups', groups: string[]): void
        (e: 'item-click', item: TreeNode): void
        (e: 'selection-change', items: string[]): void
    }

    const props = defineProps<Props>()
    const emit = defineEmits<Emits>()
    const { t } = useTranslations()

    const treeItems = computed((): TreeNode[] => {
        // Group entity definitions by group_name
        const grouped = props.entityDefinitions.reduce(
            (acc, def) => {
                if (!def.group_name) {
                    return acc
                } // Skip definitions without group
                if (!acc[def.group_name]) {
                    acc[def.group_name] = []
                }
                acc[def.group_name].push(def)
                return acc
            },
            {} as Record<string, EntityDefinition[]>
        )

        // Get entity definitions without groups
        const ungrouped = props.entityDefinitions.filter(def => !def.group_name)

        // Convert to tree structure following Vuetify's format
        const groupItems: TreeNode[] = Object.entries(grouped).map(
            ([groupName, definitions]: [string, EntityDefinition[]]) => ({
                id: `group-${groupName}`,
                title: groupName,
                entity_type: 'group',
                icon: 'mdi-folder',
                published: false,
                children: definitions.map(def => ({
                    id: def.uuid,
                    title: def.display_name,
                    uuid: def.uuid,
                    display_name: def.display_name,
                    entity_type: def.entity_type,
                    icon: def.icon || 'mdi-file-document',
                    published: def.published,
                })),
            })
        )

        // Add ungrouped items as top-level items
        const ungroupedItems: TreeNode[] = ungrouped.map(def => ({
            id: def.uuid || '',
            title: def.display_name,
            uuid: def.uuid,
            display_name: def.display_name,
            entity_type: def.entity_type,
            icon: def.icon || 'mdi-file-document',
            published: def.published,
        }))

        return [...groupItems, ...ungroupedItems]
    })

    const updateExpandedGroups = (groups: string[]) => {
        emit('update:expandedGroups', groups)
    }

    const handleItemClick = (item: TreeNode) => {
        emit('item-click', item)
    }

    const handleSelection = (items: string[]) => {
        emit('selection-change', items)
    }
</script>
