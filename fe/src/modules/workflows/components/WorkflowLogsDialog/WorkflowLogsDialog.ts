import { ref, computed, watch, defineComponent } from 'vue'
import { useTranslations } from '@/shared/composables/useTranslations'
import { typedHttpClient } from '@/api/typed-client'
import { getDialogMaxWidth } from '@/design-system/components'
import PaginatedDataTable from '@/shared/tables/PaginatedDataTable/index.vue'

export default defineComponent({
    name: 'WorkflowLogsDialog',
    components: {
        PaginatedDataTable,
    },
    props: {
        modelValue: { type: Boolean, required: true },
        runUuid: { type: String, default: null },
    },
    emits: ['update:modelValue'],
    setup(props, { emit }) {
        const { t } = useTranslations()

        const showDialog = computed({
            get: () => props.modelValue,
            set: (val) => emit('update:modelValue', val)
        })

        const logs = ref<Array<{ uuid: string; ts: string; level: string; message: string; meta?: unknown }>>([])
        const loading = ref(false)
        const page = ref(1)
        const perPage = ref(50)
        const total = ref(0)

        const loadLogs = async () => {
            if (!props.runUuid) return
            loading.value = true
            try {
                const res = await typedHttpClient.getWorkflowRunLogs(
                    props.runUuid,
                    page.value,
                    perPage.value
                )
                logs.value = res.data
                total.value = res.meta?.pagination?.total ?? res.data.length
            } finally {
                loading.value = false
            }
        }

        watch(() => props.runUuid, (newUuid) => {
            if (newUuid && props.modelValue) {
                page.value = 1
                void loadLogs()
            }
        })

        watch(() => props.modelValue, (isOpen) => {
            if (isOpen && props.runUuid) {
                void loadLogs()
            }
        })

        const handlePageChange = (p: number) => {
            page.value = p
            void loadLogs()
        }

        const handlePerPageChange = (pp: number) => {
            perPage.value = pp
            page.value = 1
            void loadLogs()
        }

        return {
            t,
            showDialog,
            logs,
            loading,
            page,
            perPage,
            total,
            handlePageChange,
            handlePerPageChange,
            getDialogMaxWidth,
        }
    }
})
