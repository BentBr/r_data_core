<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { useTranslations } from '@/composables/useTranslations'
import { usePagination } from '@/composables/usePagination'
import PaginatedDataTable from '@/components/tables/PaginatedDataTable.vue'
import CreateWorkflowDialog from '@/components/workflows/CreateWorkflowDialog.vue'
import EditWorkflowDialog from '@/components/workflows/EditWorkflowDialog.vue'
import SnackbarManager from '@/components/common/SnackbarManager.vue'
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
const runs = ref<Array<{ uuid: string; status: string; queued_at?: string | null; finished_at?: string | null; processed_items?: number | null; failed_items?: number | null }>>([])
const runsLoading = ref(false)
const runsPage = ref(1)
const runsPerPage = ref(20)
const runsTotal = ref(0)
const showLogs = ref(false)
const currentRunUuid = ref<string | null>(null)
const logs = ref<Array<{ uuid: string; ts: string; level: string; message: string }>>([])
const logsLoading = ref(false)
const logsPage = ref(1)
const logsPerPage = ref(50)
const logsTotal = ref(0)
const showCreate = ref(false)
const showEdit = ref(false)
const editingUuid = ref<string | null>(null)
const showRunDialog = ref(false)
const runTargetUuid = ref<string | null>(null)
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
const paginationMeta = ref<{ total: number; page: number; per_page: number; total_pages: number; has_previous: boolean; has_next: boolean } | null>(null)
const isComponentMounted = ref(false)

const loadWorkflows = async (page = 1, perPage = 20) => {
  if (!isComponentMounted.value) return
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
  if (!runTargetUuid.value) return
  try {
    if (uploadEnabled.value && uploadFile.value) {
      const res = await typedHttpClient.uploadRunFile(runTargetUuid.value, uploadFile.value)
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
  uploadFile.value = files && files.length ? files[0] : null
}

function editWorkflow(uuid: string) {
  editingUuid.value = uuid
  showEdit.value = true
}

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
onUnmounted(() => { isComponentMounted.value = false })

const loadRuns = async () => {
  if (!selectedWorkflowUuid.value) return
  runsLoading.value = true
  try {
    const res = selectedWorkflowUuid.value === 'all'
      ? await typedHttpClient.getAllWorkflowRuns(runsPage.value, runsPerPage.value)
      : await typedHttpClient.getWorkflowRuns(selectedWorkflowUuid.value, runsPage.value, runsPerPage.value)
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
  if (!currentRunUuid.value) return
  logsLoading.value = true
  try {
    const res = await typedHttpClient.getWorkflowRunLogs(currentRunUuid.value, logsPage.value, logsPerPage.value)
    logs.value = res.data
    logsTotal.value = res.meta?.pagination?.total ?? res.data.length
  } finally {
    logsLoading.value = false
  }
}

async function onCreated() { await load(); showCreate.value = false }
</script>

<template>
  <div class="page-wrapper">
    <div class="page">
        <div class="header">
          <h1>Workflows</h1>
          <v-spacer />
          <v-btn color="primary" @click="showCreate = true">{{ t('workflows.create.button') }}</v-btn>
        </div>

        <v-tabs v-model="activeTab">
          <v-tab value="list">List</v-tab>
          <v-tab value="history">History</v-tab>
        </v-tabs>

        <v-window v-model="activeTab" class="mt-4">
          <v-window-item value="list">
            <div>
            <PaginatedDataTable
          :items="items"
          :headers="[
            { title: 'Name', key: 'name' },
            { title: 'Kind', key: 'kind' },
            { title: 'Enabled', key: 'enabled' },
            { title: 'Cron', key: 'schedule_cron' },
            { title: 'Actions', key: 'actions' }
          ]"
          :loading="loading"
          :error="error"
          loading-text="Loading workflows..."
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
            <v-chip :color="item.enabled ? 'success' : 'error'" :text="item.enabled ? 'Enabled' : 'Disabled'" size="small" />
          </template>
          <template #item.schedule_cron="{ item }">
            {{ item.schedule_cron || '—' }}
          </template>
          <template #item.actions="{ item }">
            <v-btn size="small" variant="text" color="primary" @click="openRunNow(item.uuid)">Run now</v-btn>
            <v-btn size="small" variant="text" color="info" @click="() => { activeTab = 'history'; selectedWorkflowUuid = item.uuid; runsPage = 1; void loadRuns(); }">History</v-btn>
            <v-btn size="small" variant="text" color="secondary" @click="editWorkflow(item.uuid)">Edit</v-btn>
          </template>
            </PaginatedDataTable>
            </div>
          </v-window-item>

          <v-window-item value="history">
            <div class="d-flex align-center mb-3" style="gap: 12px;">
              <v-select
                v-model="selectedWorkflowUuid"
                :items="[{ title: 'All', value: 'all' }, ...items.map(i => ({ title: i.name, value: i.uuid }))]"
                label="Workflow"
                style="max-width: 320px;"
                @update:model-value="() => { runsPage = 1; void loadRuns(); }"
              />
              <v-spacer />
            </div>
            <PaginatedDataTable
              :items="runs"
              :headers="[
                { title: 'Status', key: 'status' },
                { title: 'Queued', key: 'queued_at' },
                { title: 'Finished', key: 'finished_at' },
                { title: 'Processed', key: 'processed_items' },
                { title: 'Failed', key: 'failed_items' },
                { title: 'Actions', key: 'actions' }
              ]"
              :loading="runsLoading"
              :error="''"
              loading-text="Loading runs..."
              :current-page="runsPage"
              :items-per-page="runsPerPage"
              :total-items="runsTotal"
              :total-pages="Math.ceil((runsTotal || 0) / runsPerPage) || 1"
              @update:page="(p:number) => { runsPage = p; void loadRuns(); }"
              @update:items-per-page="(pp:number) => { runsPerPage = pp; runsPage = 1; void loadRuns(); }"
            >
              <template #item.actions="{ item }">
                <v-btn size="small" variant="text" color="info" @click="openLogs(item.uuid)">Logs</v-btn>
              </template>
            </PaginatedDataTable>
          </v-window-item>
        </v-window>

        <v-dialog v-model="showLogs" max-width="800px">
          <v-card>
            <v-card-title>Run Logs</v-card-title>
            <v-card-text>
              <PaginatedDataTable
                :items="logs"
                :headers="[
                  { title: 'Time', key: 'ts' },
                  { title: 'Level', key: 'level' },
                  { title: 'Message', key: 'message' }
                ]"
                :loading="logsLoading"
                :error="''"
                loading-text="Loading logs..."
                :current-page="logsPage"
                :items-per-page="logsPerPage"
                :total-items="logsTotal"
                :total-pages="Math.ceil((logsTotal || 0) / logsPerPage) || 1"
                @update:page="(p:number) => { logsPage = p; void loadLogs(); }"
                @update:items-per-page="(pp:number) => { logsPerPage = pp; logsPage = 1; void loadLogs(); }"
              />
            </v-card-text>
            <v-card-actions>
              <v-spacer />
              <v-btn variant="text" @click="showLogs = false">Close</v-btn>
            </v-card-actions>
          </v-card>
        </v-dialog>

        <v-dialog v-model="showRunDialog" max-width="560px">
          <v-card>
            <v-card-title>Run workflow now</v-card-title>
            <v-card-text>
              <div class="mb-3">Confirm you want to run this workflow now. Optionally upload a CSV to process immediately.</div>
              <v-switch v-model="uploadEnabled" label="Upload CSV for this run" inset />
              <div v-if="uploadEnabled" class="mt-2">
                <input type="file" accept=".csv,text/csv" @change="onFileChange" />
                <div class="text-caption mt-1" v-if="uploadFile">Selected: {{ uploadFile.name }}</div>
              </div>
            </v-card-text>
            <v-card-actions>
              <v-spacer />
              <v-btn variant="text" @click="showRunDialog = false">Cancel</v-btn>
            <v-btn color="primary" :disabled="uploadEnabled && !uploadFile" @click="confirmRunNow">Run</v-btn>
            </v-card-actions>
          </v-card>
        </v-dialog>
      </div>
    <CreateWorkflowDialog v-model="showCreate" @created="onCreated" />
    <EditWorkflowDialog v-model="showEdit" :workflow-uuid="editingUuid" @updated="() => void loadWorkflows(currentPage, itemsPerPage)" />
    <SnackbarManager :snackbar="currentSnackbar" />
  </div>
</template>

<style scoped>
.page { padding: 16px; }
.error { color: #b00; }
.header { display: flex; align-items: center; gap: 8px; margin-bottom: 12px; }
table { width: 100%; border-collapse: collapse; }
th, td { border-bottom: 1px solid #ddd; text-align: left; padding: 8px; }
button { padding: 6px 10px; }
</style>


