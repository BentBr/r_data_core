<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { useTranslations } from '@/composables/useTranslations'
import { usePagination } from '@/composables/usePagination'
import PaginatedDataTable from '@/components/tables/PaginatedDataTable.vue'
import CreateWorkflowDialog from '@/components/workflows/CreateWorkflowDialog.vue'

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
const showCreate = ref(false)
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

async function runNow(uuid: string) {
  try {
    await typedHttpClient.runWorkflow(uuid)
  } catch (e) {
    // swallow for now
  }
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

async function onCreated() { await load(); showCreate.value = false }
</script>

<template>
  <div class="page">
    <div class="header">
      <h1>Workflows</h1>
      <v-spacer />
      <v-btn color="primary" @click="showCreate = true">{{ t('workflows.create.button') }}</v-btn>
    </div>
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
        <v-btn size="small" variant="text" color="primary" @click="runNow(item.uuid)">Run now</v-btn>
      </template>
    </PaginatedDataTable>
  </div>
  <CreateWorkflowDialog v-model="showCreate" @created="onCreated" />
  
</template>

<style scoped>
.page { padding: 16px; }
.error { color: #b00; }
.header { display: flex; align-items: center; gap: 8px; margin-bottom: 12px; }
table { width: 100%; border-collapse: collapse; }
th, td { border-bottom: 1px solid #ddd; text-align: left; padding: 8px; }
button { padding: 6px 10px; }
</style>


