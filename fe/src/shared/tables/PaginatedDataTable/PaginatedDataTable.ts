import { ref, watch, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import type { TableRow, TableHeader } from '@/types/common'

export default defineComponent({
    name: 'PaginatedDataTable',
    props: {
        items: { type: Array as PropType<TableRow[]>, required: true },
        headers: { type: Array as PropType<TableHeader[]>, required: true },
        loading: { type: Boolean, required: true },
        error: { type: String, default: undefined },
        loadingText: { type: String, default: 'Loading...' },
        currentPage: { type: Number, required: true },
        itemsPerPage: { type: Number, required: true },
        totalItems: { type: Number, required: true },
        totalPages: { type: Number, required: true },
        hasNext: { type: Boolean, default: false },
        hasPrevious: { type: Boolean, default: false },
        itemsPerPageOptions: { type: Array as PropType<number[]>, default: () => [10, 25, 50, 100, 500] },
    },
    emits: ['update:page', 'update:items-per-page', 'update:sort'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        const tableOptions = ref({
            page: props.currentPage,
            itemsPerPage: props.itemsPerPage,
            sortBy: [] as Array<{ key: string; order: 'asc' | 'desc' }>,
        })

        const lastEmittedSort = ref<{ key: string | null; order: 'asc' | 'desc' | null }>({
            key: null,
            order: null,
        })

        watch(() => [props.currentPage, props.itemsPerPage], ([page, itemsPerPage]) => {
            tableOptions.value = { ...tableOptions.value, page: page as number, itemsPerPage: itemsPerPage as number }
        }, { immediate: true })

        const handleOptionsUpdate = (options: { page: number, itemsPerPage: number, sortBy?: Array<{ key: string; order: 'asc' | 'desc' }> }) => {
            const nextSort = options.sortBy?.[0]
            const nextSortKey = nextSort?.key ?? null
            const nextSortOrder = nextSort?.order ?? null
            const pageChanged = options.page !== props.currentPage
            const itemsPerPageChanged = options.itemsPerPage !== props.itemsPerPage
            const sortChanged = nextSortKey !== lastEmittedSort.value.key || nextSortOrder !== lastEmittedSort.value.order

            if (pageChanged) emit('update:page', options.page)
            if (itemsPerPageChanged) emit('update:items-per-page', options.itemsPerPage)
            if (sortChanged) {
                emit('update:sort', nextSortKey, nextSortOrder)
                lastEmittedSort.value = { key: nextSortKey, order: nextSortOrder }
            }
            tableOptions.value = { page: options.page, itemsPerPage: options.itemsPerPage, sortBy: options.sortBy ?? [] }
        }

        return { t, tableOptions, handleOptionsUpdate }
    },
})
