import { ref, onMounted, defineComponent } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { typedHttpClient } from '@/api/typed-client'
import PaginatedDataTable from '@/shared/tables/PaginatedDataTable/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import Badge from '@/shared/components/Badge/index.vue'
import { usePagination } from '@/shared/composables/usePagination'

type WorkflowSummary = {
    uuid: string
    name: string
    kind: 'consumer' | 'provider'
    enabled: boolean
    schedule_cron?: string | null
    has_api_endpoint?: boolean
}

export default defineComponent({
    name: 'WorkflowListTab',
    components: {
        PaginatedDataTable,
        SmartIcon,
        Badge,
    },
    props: {
        loading: { type: Boolean, default: false },
    },
    emits: ['run', 'history', 'edit', 'delete'],
    setup(_, { emit }) {
        const { t } = useTranslations()

        const items = ref<WorkflowSummary[]>([])
        const error = ref('')
        const sortBy = ref<string | null>(null)
        const sortOrder = ref<'asc' | 'desc' | null>(null)

        const { state: paginationState, setPage, setItemsPerPage } = usePagination('workflows_list', 20)
        const currentPage = ref(paginationState.page)
        const itemsPerPage = ref(paginationState.itemsPerPage)
        const totalItems = ref(0)
        const totalPages = ref(1)
        const paginationMeta = ref<any>(null)

        const loadWorkflows = async () => {
            error.value = ''
            try {
                const response = await typedHttpClient.getWorkflows(
                    currentPage.value,
                    itemsPerPage.value,
                    sortBy.value,
                    sortOrder.value
                )
                items.value = response.data.map((item: any) => ({
                    ...item,
                    kind: item.kind.toLowerCase() as 'consumer' | 'provider',
                }))
                if (response.meta?.pagination) {
                    totalItems.value = response.meta.pagination.total
                    totalPages.value = response.meta.pagination.total_pages
                    paginationMeta.value = response.meta.pagination
                } else {
                    totalItems.value = items.value.length
                    totalPages.value = 1
                }
            } catch (e: any) {
                error.value = e.message || String(e)
            }
        }

        const handlePageChange = async (page: number) => {
            currentPage.value = page
            setPage(page)
            await loadWorkflows()
        }

        const handleItemsPerPageChange = async (newItemsPerPage: number) => {
            itemsPerPage.value = newItemsPerPage
            setItemsPerPage(newItemsPerPage)
            currentPage.value = 1
            await loadWorkflows()
        }

        const handleSortChange = async (newSortBy: string | null, newSortOrder: 'asc' | 'desc' | null) => {
            sortBy.value = newSortBy
            sortOrder.value = newSortOrder
            currentPage.value = 1
            await loadWorkflows()
        }

        onMounted(() => { void loadWorkflows() })

        return {
            t,
            items,
            error,
            currentPage,
            itemsPerPage,
            totalItems,
            totalPages,
            paginationMeta,
            handlePageChange,
            handleItemsPerPageChange,
            handleSortChange,
            refresh: loadWorkflows,
            emit,
        }
    }
})
