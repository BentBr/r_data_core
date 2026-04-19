import { defineComponent, PropType } from 'vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import Badge from '@/shared/components/Badge/index.vue'
import { buttonConfigs } from '@/design-system/components'
import type { TableRow, TableHeader, TableAction } from '@/types/common'

export default defineComponent({
    name: 'DataTable',
    components: {
        SmartIcon,
        Badge,
    },
    props: {
        headers: { type: Array as PropType<TableHeader[]>, required: true },
        items: { type: Array as PropType<TableRow[]>, required: true },
        loading: { type: Boolean, default: false },
        itemsPerPage: { type: Number, default: 10 },
        page: { type: Number, default: 1 },
        totalItems: { type: Number, default: 0 },
        actions: { type: Array as PropType<TableAction[]>, default: () => [] },
    },
    emits: ['update:options'],
    setup(_, { emit }) {
        const handleOptionsUpdate = (options: any) => { emit('update:options', options) }
        const row = (item: any): TableRow => item as TableRow
        const formatDate = (dateString?: string | null) => dateString ? new Date(dateString).toLocaleDateString() : 'N/A'
        return { handleOptionsUpdate, row, formatDate, buttonConfigs }
    },
})
