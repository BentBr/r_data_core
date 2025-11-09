<script setup lang="ts">
    import { ref, computed, onMounted, onUnmounted } from 'vue'
    import { typedHttpClient } from '@/api/typed-client'
    import { useTranslations } from '@/composables/useTranslations'
    import { usePagination } from '@/composables/usePagination'
    import PaginatedDataTable from '@/components/tables/PaginatedDataTable.vue'
    import CreateWorkflowDialog from '@/components/workflows/CreateWorkflowDialog.vue'
    import EditWorkflowDialog from '@/components/workflows/EditWorkflowDialog.vue'
    import SnackbarManager from '@/components/common/SnackbarManager.vue'
    import DialogManager from '@/components/common/DialogManager.vue'
    import { useSnackbar } from '@/composables/useSnackbar'

    type WorkflowSummary = {
        uuid: string
        name: string
        kind: 'consumer' | 'provider'
        enabled: boolean
        schedule_cron?: string | null
    }

    const loading = ref(false)
    const items = ref<WorkflowSummary[]>([])
    const error = ref('')
    // History state
    const activeTab = ref<'list' | 'history'>('list')
    const selectedWorkflowUuid = ref<string | null>(null)
    const runs = ref<
        Array<{
            uuid: string
            status: string
            queued_at?: string | null
            finished_at?: string | null
            processed_items?: number | null
            failed_items?: number | null
        }>
    >([])
    const runsLoading = ref(false)
    const runsPage = ref(1)
    const runsPerPage = ref(20)
    const runsTotal = ref(0)
    const showLogs = ref(false)
    const currentRunUuid = ref<string | null>(null)
    const logs = ref<
        Array<{ uuid: string; ts: string; level: string; message: string; meta?: unknown }>
    >([])
    const logsLoading = ref(false)
    const logsPage = ref(1)
    const logsPerPage = ref(50)
    const logsTotal = ref(0)
    const showCreate = ref(false)
    const showEdit = ref(false)
    const editingUuid = ref<string | null>(null)
    const showRunDialog = ref(false)
    const runTargetUuid = ref<string | null>(null)
    const showDeleteDialog = ref(false)
    const workflowToDelete = ref<WorkflowSummary | null>(null)
    const deleting = ref(false)
    const uploadEnabled = ref(false)
    const uploadFile = ref<File | null>(null)
    const { currentSnackbar, showSuccess, showError } = useSnackbar()
    const { t } = useTranslations()

    // Pagination
    const { state: paginationState, setPage, setItemsPerPage } = usePagination('workflows', 20)
    const currentPage = ref(paginationState.page)
    const itemsPerPage = ref(paginationState.itemsPerPage)
    const totalItems = ref(0)
    const totalPages = ref(1)
    const paginationMeta = ref<{
        total: number
        page: number
        per_page: number
        total_pages: number
        has_previous: boolean
        has_next: boolean
    } | null>(null)
    const isComponentMounted = ref(false)

    const loadWorkflows = async (page = 1, perPage = 20) => {
        if (!isComponentMounted.value) {
            return
        }
        loading.value = true
        error.value = ''
        try {
            const response = await typedHttpClient.getWorkflows(page, perPage)
            items.value = response.data
            if (response.meta?.pagination) {
                totalItems.value = response.meta.pagination.total
                totalPages.value = response.meta.pagination.total_pages
                paginationMeta.value = response.meta.pagination
            } else {
                totalItems.value = items.value.length
                totalPages.value = 1
                paginationMeta.value = null
            }
        } catch (e: any) {
            error.value = e?.message ?? 'Failed to load workflows'
        } finally {
            loading.value = false
        }
    }

    function openRunNow(uuid: string) {
        runTargetUuid.value = uuid
        uploadEnabled.value = false
        uploadFile.value = null
        showRunDialog.value = true
    }

    async function confirmRunNow() {
        if (!runTargetUuid.value) {
            return
        }
        try {
            if (uploadEnabled.value && uploadFile.value) {
                const res = await typedHttpClient.uploadRunFile(
                    runTargetUuid.value,
                    uploadFile.value
                )
                showSuccess(`Run enqueued (staged ${res.staged_items})`)
            } else {
                await typedHttpClient.runWorkflow(runTargetUuid.value)
                showSuccess('Workflow run enqueued')
            }
            showRunDialog.value = false
        } catch (e) {
            showError(e instanceof Error ? e.message : 'Failed to enqueue run')
        }
    }

    function onFileChange(e: Event) {
        const input = e.target as HTMLInputElement | null
        const files = input?.files
        uploadFile.value = files?.length ? files[0] : null
    }

    function editWorkflow(uuid: string) {
        editingUuid.value = uuid
        showEdit.value = true
    }

    function confirmDeleteWorkflow(item: WorkflowSummary) {
        workflowToDelete.value = item
        showDeleteDialog.value = true
    }

    async function deleteWorkflow() {
        if (!workflowToDelete.value) {
            return
        }

        deleting.value = true

        try {
            await typedHttpClient.deleteWorkflow(workflowToDelete.value.uuid)
            showSuccess(t('workflows.delete.success'))
            showDeleteDialog.value = false
            workflowToDelete.value = null
            await loadWorkflows(currentPage.value, itemsPerPage.value)
        } catch (e) {
            showError(e instanceof Error ? e.message : t('workflows.delete.error'))
        } finally {
            deleting.value = false
        }
    }

    const deleteDialogConfig = computed(() => ({
        title: t('workflows.delete.title'),
        maxWidth: '500px',
        persistent: false,
    }))

    const handlePageChange = async (page: number) => {
        currentPage.value = page
        setPage(page)
        await loadWorkflows(currentPage.value, itemsPerPage.value)
    }

    const handleItemsPerPageChange = async (newItemsPerPage: number) => {
        itemsPerPage.value = newItemsPerPage
        setItemsPerPage(newItemsPerPage)
        currentPage.value = 1
        setPage(1)
        await loadWorkflows(1, newItemsPerPage)
    }

    onMounted(() => {
        isComponentMounted.value = true
        void loadWorkflows(currentPage.value, itemsPerPage.value)
    })
    onUnmounted(() => {
        isComponentMounted.value = false
    })

    const loadRuns = async () => {
        if (!selectedWorkflowUuid.value) {
            return
        }
        runsLoading.value = true
        try {
            const res =
                selectedWorkflowUuid.value === 'all'
                    ? await typedHttpClient.getAllWorkflowRuns(runsPage.value, runsPerPage.value)
                    : await typedHttpClient.getWorkflowRuns(
                          selectedWorkflowUuid.value,
                          runsPage.value,
                          runsPerPage.value
                      )
            runs.value = res.data
            runsTotal.value = res.meta?.pagination?.total ?? res.data.length
        } finally {
            runsLoading.value = false
        }
    }

    const openLogs = async (runUuid: string) => {
        currentRunUuid.value = runUuid
        logsPage.value = 1
        showLogs.value = true
        await loadLogs()
    }

    const loadLogs = async () => {
        if (!currentRunUuid.value) {
            return
        }
        logsLoading.value = true
        try {
            const res = await typedHttpClient.getWorkflowRunLogs(
                currentRunUuid.value,
                logsPage.value,
                logsPerPage.value
            )
            logs.value = res.data
            logsTotal.value = res.meta?.pagination?.total ?? res.data.length
        } finally {
            logsLoading.value = false
        }
    }

    async function onCreated() {
        await loadWorkflows(currentPage.value, itemsPerPage.value)
        showCreate.value = false
    }
</script>

<script lang="ts">
    export default {
        // Expose selected internals for stable tests
        expose: [
            'openRunNow',
            'confirmRunNow',
            'uploadEnabled',
            'uploadFile',
            'activeTab',
            'selectedWorkflowUuid',
            'loadRuns',
            'showRunDialog',
        ],
    }
</script>

<template>
    <div class="page-wrapper">
        <div class="page">
            <div class="header">
                <h1>{{ t('navigation.workflows') }}</h1>
                <v-spacer />
                <v-btn
                    color="primary"
                    @click="showCreate = true"
                    >{{ t('workflows.create.button') }}</v-btn
                >
            </div>

            <v-tabs v-model="activeTab">
                <v-tab value="list">{{ t('table.list') ?? 'List' }}</v-tab>
                <v-tab value="history">{{ t('workflows.history.tab') ?? 'History' }}</v-tab>
            </v-tabs>

            <v-window
                v-model="activeTab"
                class="mt-4"
            >
                <v-window-item value="list">
                    <div>
                        <PaginatedDataTable
                            :items="items"
                            :headers="[
                                { title: t('workflows.table.name') || 'Name', key: 'name' },
                                { title: t('workflows.table.kind') || 'Kind', key: 'kind' },
                                {
                                    title: t('workflows.table.enabled') || 'Enabled',
                                    key: 'enabled',
                                },
                                {
                                    title: t('workflows.table.cron') || 'Cron',
                                    key: 'schedule_cron',
                                },
                                {
                                    title: t('workflows.table.actions') || 'Actions',
                                    key: 'actions',
                                },
                            ]"
                            :loading="loading"
                            :error="error"
                            :loading-text="t('table.loading')"
                            :current-page="currentPage"
                            :items-per-page="itemsPerPage"
                            :total-items="totalItems"
                            :total-pages="totalPages"
                            :has-next="paginationMeta?.has_next"
                            :has-previous="paginationMeta?.has_previous"
                            @update:page="handlePageChange"
                            @update:items-per-page="handleItemsPerPageChange"
                        >
                            <template #item.enabled="{ item }">
                                <v-chip
                                    :color="item.enabled ? 'success' : 'error'"
                                    :text="item.enabled ? 'Enabled' : 'Disabled'"
                                    size="small"
                                />
                            </template>
                            <template #item.schedule_cron="{ item }">
                                {{ item.schedule_cron || t('common.empty') }}
                            </template>
                            <template #item.actions="{ item }">
                                <v-btn
                                    icon="mdi-play-circle"
                                    variant="text"
                                    color="primary"
                                    :title="t('workflows.actions.run_now')"
                                    @click="openRunNow(item.uuid)"
                                />
                                <v-btn
                                    icon="mdi-history"
                                    variant="text"
                                    color="info"
                                    :title="t('workflows.actions.history')"
                                    @click="
                                        () => {
                                            activeTab = 'history'
                                            selectedWorkflowUuid = item.uuid
                                            runsPage = 1
                                            void loadRuns()
                                        }
                                    "
                                />
                                <v-btn
                                    icon="mdi-pencil"
                                    variant="text"
                                    color="secondary"
                                    :title="t('common.edit')"
                                    @click="editWorkflow(item.uuid)"
                                />
                                <v-btn
                                    icon="mdi-delete"
                                    variant="text"
                                    color="error"
                                    :title="t('workflows.actions.delete')"
                                    @click="confirmDeleteWorkflow(item)"
                                />
                            </template>
                        </PaginatedDataTable>
                    </div>
                </v-window-item>

                <v-window-item value="history">
                    <div
                        class="d-flex align-center mb-3"
                        style="gap: 12px"
                    >
                        <v-select
                            v-model="selectedWorkflowUuid"
                            :items="[
                                { title: t('workflows.history.all'), value: 'all' },
                                ...items.map(i => ({ title: i.name, value: i.uuid })),
                            ]"
                            :label="t('workflows.history.select')"
                            style="max-width: 320px"
                            @update:model-value="
                                () => {
                                    runsPage = 1
                                    void loadRuns()
                                }
                            "
                        />
                        <v-spacer />
                    </div>
                    <PaginatedDataTable
                        :items="runs"
                        :headers="[
                            { title: t('workflows.history.status'), key: 'status' },
                            { title: t('workflows.history.queued'), key: 'queued_at' },
                            { title: t('workflows.history.finished'), key: 'finished_at' },
                            { title: t('workflows.history.processed'), key: 'processed_items' },
                            { title: t('workflows.history.failed'), key: 'failed_items' },
                            { title: t('workflows.table.actions'), key: 'actions' },
                        ]"
                        :loading="runsLoading"
                        :error="''"
                        :loading-text="t('table.loading')"
                        :current-page="runsPage"
                        :items-per-page="runsPerPage"
                        :total-items="runsTotal"
                        :total-pages="Math.ceil((runsTotal || 0) / runsPerPage) || 1"
                        @update:page="
                            (p: number) => {
                                runsPage = p
                                void loadRuns()
                            }
                        "
                        @update:items-per-page="
                            (pp: number) => {
                                runsPerPage = pp
                                runsPage = 1
                                void loadRuns()
                            }
                        "
                    >
                        <template #item.actions="{ item }">
                            <v-btn
                                icon="mdi-text"
                                variant="text"
                                color="info"
                                :title="t('workflows.history.logs')"
                                @click="openLogs(item.uuid)"
                            />
                        </template>
                    </PaginatedDataTable>
                </v-window-item>
            </v-window>

            <v-dialog
                v-model="showLogs"
                max-width="1200px"
            >
                <v-card>
                    <v-card-title>{{ t('workflows.history.logs') }}</v-card-title>
                    <v-card-text>
                        <PaginatedDataTable
                            :items="logs"
                            :headers="[
                                { title: t('workflows.logs.time'), key: 'ts' },
                                { title: t('workflows.logs.level'), key: 'level' },
                                { title: t('workflows.logs.message'), key: 'message' },
                                { title: t('workflows.logs.meta'), key: 'meta' },
                            ]"
                            :loading="logsLoading"
                            :error="''"
                            :loading-text="t('table.loading')"
                            :current-page="logsPage"
                            :items-per-page="logsPerPage"
                            :total-items="logsTotal"
                            :total-pages="Math.ceil((logsTotal || 0) / logsPerPage) || 1"
                            @update:page="
                                (p: number) => {
                                    logsPage = p
                                    void loadLogs()
                                }
                            "
                            @update:items-per-page="
                                (pp: number) => {
                                    logsPerPage = pp
                                    logsPage = 1
                                    void loadLogs()
                                }
                            "
                        >
                            <template #item.meta="{ item }">
                                <pre
                                    style="
                                        white-space: pre-wrap;
                                        word-break: break-word;
                                        font-size: 12px;
                                        margin: 0;
                                    "
                                    >{{
                                        typeof item.meta === 'string'
                                            ? item.meta
                                            : JSON.stringify(item.meta ?? {}, null, 2)
                                    }}
                                </pre>
                            </template>
                        </PaginatedDataTable>
                    </v-card-text>
                    <v-card-actions>
                        <v-spacer />
                        <v-btn
                            variant="text"
                            @click="showLogs = false"
                            >{{ t('common.close') }}</v-btn
                        >
                    </v-card-actions>
                </v-card>
            </v-dialog>

            <v-dialog
                v-model="showRunDialog"
                max-width="560px"
            >
                <v-card>
                    <v-card-title>{{ t('workflows.run.confirm_title') }}</v-card-title>
                    <v-card-text>
                        <div class="mb-3">{{ t('workflows.run.confirm_message_simple') }}</div>
                        <v-switch
                            v-model="uploadEnabled"
                            :label="t('workflows.run.upload_csv_toggle')"
                            color="success"
                            inset
                        />
                        <div
                            v-if="uploadEnabled"
                            class="mt-2"
                        >
                            <input
                                type="file"
                                accept=".csv,text/csv"
                                @change="onFileChange"
                            />
                            <div
                                v-if="uploadFile"
                                class="text-caption mt-1"
                            >
                                {{ t('workflows.run.selected_file') }}: {{ uploadFile.name }}
                            </div>
                        </div>
                    </v-card-text>
                    <v-card-actions>
                        <v-spacer />
                        <v-btn
                            variant="text"
                            @click="showRunDialog = false"
                            >{{ t('common.cancel') }}</v-btn
                        >
                        <v-btn
                            color="primary"
                            :disabled="uploadEnabled && !uploadFile"
                            @click="confirmRunNow"
                            >{{ t('workflows.run.run_button') }}</v-btn
                        >
                    </v-card-actions>
                </v-card>
            </v-dialog>
        </div>
        <CreateWorkflowDialog
            v-model="showCreate"
            @created="onCreated"
        />
        <EditWorkflowDialog
            v-model="showEdit"
            :workflow-uuid="editingUuid"
            @updated="() => void loadWorkflows(currentPage, itemsPerPage)"
        />
        <DialogManager
            v-model="showDeleteDialog"
            :config="deleteDialogConfig"
            :loading="deleting"
            :confirm-text="t('workflows.delete.button')"
            :cancel-text="t('common.cancel')"
            @confirm="deleteWorkflow"
        >
            <p>{{ t('workflows.delete.confirm_message') }}</p>
        </DialogManager>
        <SnackbarManager :snackbar="currentSnackbar" />
    </div>
</template>

<style scoped>
    .page {
        padding: 16px;
    }
    .error {
        color: #b00;
    }
    .header {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-bottom: 12px;
    }
    table {
        width: 100%;
        border-collapse: collapse;
    }
    th,
    td {
        border-bottom: 1px solid #ddd;
        text-align: left;
        padding: 8px;
    }
    button {
        padding: 6px 10px;
    }
</style>
