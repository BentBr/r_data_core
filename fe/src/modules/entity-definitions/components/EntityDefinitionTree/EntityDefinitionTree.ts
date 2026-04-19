import { computed, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import TreeView from '@/shared/components/TreeView/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import type { EntityDefinition, TreeNode } from '@/types/schemas'

export default defineComponent({
    name: 'EntityDefinitionTree',
    components: {
        TreeView,
        SmartIcon,
    },
    props: {
        entityDefinitions: { type: Array as PropType<EntityDefinition[]>, required: true },
        loading: { type: Boolean, required: true },
        expandedGroups: { type: Array as PropType<string[]>, required: true },
    },
    emits: ['update:expandedGroups', 'item-click', 'selection-change'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        const treeItems = computed((): TreeNode[] => {
            const grouped = props.entityDefinitions.reduce<Record<string, EntityDefinition[]>>((acc, def) => {
                if (!def.group_name) return acc
                const group = acc[def.group_name] ?? []
                group.push(def); acc[def.group_name] = group; return acc
            }, {})
            const ungrouped = props.entityDefinitions.filter(def => !def.group_name)
            const groupItems: TreeNode[] = Object.entries(grouped).map(([groupName, definitions]) => ({
                id: `group-${groupName}`, title: groupName, entity_type: 'group', icon: 'folder', published: false,
                children: definitions.map(def => ({ id: def.uuid ?? '', title: def.display_name, uuid: def.uuid, display_name: def.display_name, entity_type: def.entity_type, icon: def.icon ?? 'file-text', published: def.published }))
            }))
            const ungroupedItems: TreeNode[] = ungrouped.map(def => ({ id: def.uuid ?? '', title: def.display_name, uuid: def.uuid, display_name: def.display_name, entity_type: def.entity_type, icon: def.icon ?? 'file-text', published: def.published }))
            return [...groupItems, ...ungroupedItems]
        })

        return {
            t, treeItems,
            updateExpandedGroups: (groups: string[]) => emit('update:expandedGroups', groups),
            handleItemClick: (item: TreeNode) => emit('item-click', item),
            handleSelection: (items: string[]) => emit('selection-change', items),
        }
    },
})
