import { ref, watch, defineComponent, PropType } from 'vue'
import SmartIcon from '../SmartIcon/index.vue'
import type { TreeNode } from '@/types/schemas'

export default defineComponent({
    name: 'TreeView',
    components: {
        SmartIcon,
    },
    props: {
        items: { type: Array as PropType<TreeNode[]>, required: true },
        loading: { type: Boolean, default: false },
        expandedItems: { type: Array as PropType<string[]>, default: () => [] },
    },
    emits: ['update:expandedItems', 'item-click', 'selection-change'],
    setup(props, { emit }) {
        const expandedItems = ref<string[]>(props.expandedItems)
        const syncingFromProps = ref(false)

        const isUnpublished = (item: TreeNode) => {
            if (!item.entity_type || item.entity_type === 'group') return false
            const val = item.published as any
            return val === false || val === null || val === 0 || val === 'false'
        }

        watch(() => props.expandedItems, v => {
            syncingFromProps.value = true; expandedItems.value = v
            queueMicrotask(() => { syncingFromProps.value = false })
        })

        watch(expandedItems, newValue => { emit('update:expandedItems', newValue) })

        const handleSelection = (items: string[]) => { emit('selection-change', items) }
        const handleItemClick = (item: TreeNode) => { emit('item-click', item) }
        const hasChildren = (item: TreeNode): boolean => item.children !== undefined && Array.isArray(item.children)
        const toggleItem = (item: TreeNode) => {
            const index = expandedItems.value.indexOf(item.id)
            if (index > -1) expandedItems.value = expandedItems.value.filter(id => id !== item.id)
            else expandedItems.value = [...expandedItems.value, item.id]
        }

        return {
            expandedItems, isUnpublished, handleSelection, handleItemClick, hasChildren, toggleItem,
        }
    },
})
