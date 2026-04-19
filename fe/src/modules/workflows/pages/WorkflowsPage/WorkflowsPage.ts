import { ref, computed, onMounted, nextTick, defineComponent } from 'vue'
import { useRoute } from 'vue-router'
import { typedHttpClient } from '@/api/typed-client'
import { useTranslations } from '@/shared/composables/useTranslations'
import PageLayout from '@/shared/components/PageLayout/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import CreateWorkflowDialog from '@/modules/workflows/components/CreateWorkflowDialog/index.vue'
import EditWorkflowDialog from '@/modules/workflows/components/EditWorkflowDialog/index.vue'
import WorkflowRunDialog from '@/modules/workflows/components/WorkflowRunDialog/index.vue'
import WorkflowListTab from '@/modules/workflows/components/page-tabs/WorkflowListTab/index.vue'
import WorkflowHistoryTab from '@/modules/workflows/components/page-tabs/WorkflowHistoryTab/index.vue'
import DialogManager from '@/shared/components/DialogManager/index.vue'
import SnackbarManager from '@/shared/components/SnackbarManager/index.vue'
import { useSnackbar } from '@/shared/composables/useSnackbar'
import { useErrorHandler } from '@/shared/composables/useErrorHandler'
import { useAuthStore } from '@/stores/auth'

export default defineComponent({
    name: 'WorkflowsPage',
    components: {
        PageLayout,
        SmartIcon,
        CreateWorkflowDialog,
        EditWorkflowDialog,
        WorkflowRunDialog,
        WorkflowListTab,
        WorkflowHistoryTab,
        DialogManager,
        SnackbarManager,
    },
    setup() {
        const authStore = useAuthStore()
        const { currentSnackbar, showSuccess } = useSnackbar()
        const { handleError } = useErrorHandler()
        const { t } = useTranslations()
        const route = useRoute()

        const listTabRef = ref<any>(null)
        const historyTabRef = ref<any>(null)
        const runDialogRef = ref<any>(null)
        const activeTab = ref<'list' | 'history'>('list')
        const showCreate = ref(false)
        const showEdit = ref(false)
        const showRunDialog = ref(false)
        const showDeleteDialog = ref(false)
        
        const editingUuid = ref<string | null>(null)
        const runTargetUuid = ref<string | null>(null)
        const workflowToDelete = ref<any>(null)
        const deleting = ref(false)
        const historyInitialUuid = ref('all')

        const canCreateWorkflow = computed(() => {
            return authStore.hasPermission('Workflows', 'Create') || authStore.hasPermission('Workflows', 'Admin')
        })

        const deleteDialogConfig = computed(() => ({
            title: t('workflows.delete.title'),
            maxWidth: '500px',
            persistent: false,
        }))

        const openRunNow = (uuid: string) => { runTargetUuid.value = uuid; showRunDialog.value = true }
        const openHistory = (uuid: string) => { historyInitialUuid.value = uuid; activeTab.value = 'history' }
        const confirmRunNow = async () => { await runDialogRef.value?.confirmRun() }

        const selectedWorkflowUuid = computed({
            get: () => historyTabRef.value?.selectedWorkflowUuid ?? historyInitialUuid.value,
            set: (value: string) => {
                if (historyTabRef.value) historyTabRef.value.selectedWorkflowUuid = value
                else historyInitialUuid.value = value
            },
        })

        const loadRuns = async () => { await historyTabRef.value?.loadRuns?.() }

        const uploadEnabled = computed({
            get: () => runDialogRef.value?.uploadEnabled ?? false,
            set: (value: boolean) => {
                if (runDialogRef.value) runDialogRef.value.uploadEnabled = value
            },
        })

        const uploadFile = computed({
            get: () => runDialogRef.value?.uploadFile ?? null,
            set: (value: File | null) => {
                if (runDialogRef.value) runDialogRef.value.uploadFile = value
            },
        })
        const openEdit = (uuid: string) => { editingUuid.value = uuid; showEdit.value = true }
        const openDelete = (item: any) => { workflowToDelete.value = item; showDeleteDialog.value = true }

        const deleteWorkflow = async () => {
            if (!workflowToDelete.value) return
            deleting.value = true
            try {
                await typedHttpClient.deleteWorkflow(workflowToDelete.value.uuid)
                showSuccess(t('workflows.delete.success'))
                showDeleteDialog.value = false
                workflowToDelete.value = null
                listTabRef.value?.refresh()
            } catch (e) { handleError(e) } 
            finally { deleting.value = false }
        }

        const onCreated = () => { showCreate.value = false; listTabRef.value?.refresh() }
        const onUpdated = () => { showEdit.value = false; listTabRef.value?.refresh() }

        onMounted(async () => {
            if (route.query.tab === 'history') activeTab.value = 'history'
            if (route.query.create === 'true') {
                await nextTick()
                showCreate.value = true
                window.history.replaceState({}, '', '/workflows')
            }
        })

        return {
            t, activeTab, showCreate, showEdit, showRunDialog, showDeleteDialog,
            editingUuid, runTargetUuid, historyInitialUuid, workflowToDelete, deleting, deleteDialogConfig,
            canCreateWorkflow, listTabRef, historyTabRef, runDialogRef, currentSnackbar,
            selectedWorkflowUuid, uploadEnabled, uploadFile,
            openRunNow, openHistory, openEdit, openDelete, confirmRunNow, loadRuns,
            deleteWorkflow, onCreated, onUpdated,
        }
    },
})
