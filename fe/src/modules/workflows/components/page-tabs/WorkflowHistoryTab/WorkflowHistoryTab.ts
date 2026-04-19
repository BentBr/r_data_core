import { ref, watch, defineComponent, PropType } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { typedHttpClient } from '@/api/typed-client'
import PaginatedDataTable from '@/shared/tables/PaginatedDataTable/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import WorkflowLogsDialog from '../../WorkflowLogsDialog/index.vue'

type WorkflowSummary = {
    uuid: string
    name: string
}

export default defineComponent({
    name: 'WorkflowHistoryTab',
    components: {
        PaginatedDataTable,
        SmartIcon,
        WorkflowLogsDialog,
    },
    props: {
        workflows: { type: Array as PropType<WorkflowSummary[]>, required: true },
        initialWorkflowUuid: { type: String, default: 'all' },
    },
    setup(props) {
        const { t } = useTranslations()

        const selectedWorkflowUuid = ref(props.initialWorkflowUuid)
        const runs = ref<any[]>([])
        const loading = ref(false)
        const page = ref(1)
        const perPage = ref(20)
        const total = ref(0)

        const showLogs = ref(false)
        const currentRunUuid = ref<string | null>(null)

        const loadRuns = async () => {
            if (!selectedWorkflowUuid.value) return
            loading.value = true
            try {
                const res = selectedWorkflowUuid.value === 'all'
                    ? await typedHttpClient.getAllWorkflowRuns(page.value, perPage.value)
                    : await typedHttpClient.getWorkflowRuns(selectedWorkflowUuid.value, page.value, perPage.value)
                runs.value = res.data
                total.value = res.meta?.pagination?.total ?? res.data.length
            } finally {
                loading.value = false
            }
        }

        watch(() => selectedWorkflowUuid.value, () => {
            page.value = 1
            void loadRuns()
        })

        const openLogs = (runUuid: string) => {
            currentRunUuid.value = runUuid
            showLogs.value = true
        }

        const handlePageChange = (p: number) => {
            page.value = p
            void loadRuns()
        }

        const handlePerPageChange = (pp: number) => {
            perPage.value = pp
            page.value = 1
            void loadRuns()
        }

        // Load initially
        void loadRuns()

        return {
            t,
            selectedWorkflowUuid,
            runs,
            loading,
            page,
            perPage,
            total,
            showLogs,
            currentRunUuid,
            openLogs,
            loadRuns,
            handlePageChange,
            handlePerPageChange,
        }
    }
})
