<template>
    <div data-testid="system-logs-viewer">
        <div class="d-flex align-center justify-space-between pa-4">
            <h3 class="text-h6">{{ t('system.logs.title') }}</h3>
        </div>

        <!-- Filter bar -->
        <div class="d-flex gap-3 px-4 pb-4 flex-wrap">
            <v-select
                v-model="filterLogType"
                :items="logTypeOptions"
                :label="t('system.logs.filter_type')"
                clearable
                density="compact"
                variant="outlined"
                style="min-width: 200px; max-width: 240px"
                @update:model-value="handleFilterChange"
            />
            <v-select
                v-model="filterResourceType"
                :items="resourceTypeOptions"
                :label="t('system.logs.filter_resource')"
                clearable
                density="compact"
                variant="outlined"
                style="min-width: 200px; max-width: 240px"
                @update:model-value="handleFilterChange"
            />
            <v-select
                v-model="filterStatus"
                :items="statusOptions"
                :label="t('system.logs.filter_status')"
                clearable
                density="compact"
                variant="outlined"
                style="min-width: 180px; max-width: 220px"
                @update:model-value="handleFilterChange"
            />
        </div>

        <v-data-table-server
            :items="logs"
            :headers="headers"
            :loading="loading"
            :items-length="total"
            :items-per-page="itemsPerPage"
            :page="page"
            :no-data-text="t('system.logs.no_logs')"
            :loading-text="t('table.loading')"
            data-testid="system-logs-table"
            @update:page="handlePageChange"
            @update:items-per-page="handleItemsPerPageChange"
        >
            <template #item.created_at="{ item }">
                <span class="text-body-2">{{ formatDate(item.created_at) }}</span>
            </template>

            <template #item.log_type="{ item }">
                <v-chip
                    :color="getLogTypeColor(item.log_type)"
                    size="small"
                    variant="tonal"
                >
                    {{ item.log_type }}
                </v-chip>
            </template>

            <template #item.resource_type="{ item }">
                <span class="text-body-2">{{ item.resource_type }}</span>
            </template>

            <template #item.resource_uuid="{ item }">
                <router-link
                    v-if="item.resource_uuid && resourceRoutes[item.resource_type]"
                    :to="resourceRoutes[item.resource_type]"
                    class="text-body-2"
                >
                    {{ item.resource_uuid.slice(0, 8) }}…
                </router-link>
                <span
                    v-else-if="item.resource_uuid"
                    class="text-body-2 text-medium-emphasis"
                >
                    {{ item.resource_uuid.slice(0, 8) }}…
                </span>
                <span
                    v-else
                    class="text-medium-emphasis"
                >
                    —
                </span>
            </template>

            <template #item.summary="{ item }">
                <span
                    class="text-body-2"
                    style="cursor: pointer"
                    @click="item.log_type === 'email_sent' ? openEmailPreview(item) : undefined"
                >
                    {{ item.summary }}
                </span>
            </template>

            <template #item.status="{ item }">
                <v-chip
                    :color="getStatusColor(item.status)"
                    size="small"
                    variant="tonal"
                >
                    {{ item.status }}
                </v-chip>
            </template>
        </v-data-table-server>

        <!-- Email preview dialog -->
        <v-dialog
            v-model="showEmailPreview"
            max-width="800px"
        >
            <v-card>
                <v-card-title class="pa-6">
                    {{ t('system.logs.email_preview') }}
                </v-card-title>
                <v-card-text class="pa-6">
                    <div
                        v-if="previewLog?.details"
                        class="email-preview-frame"
                        v-html="getHtmlPreview(previewLog.details)"
                    />
                    <div
                        v-else
                        class="text-medium-emphasis"
                    >
                        {{ t('system.logs.no_logs') }}
                    </div>
                </v-card-text>
                <v-card-actions class="pa-4 px-6">
                    <v-spacer />
                    <v-btn
                        variant="text"
                        @click="showEmailPreview = false"
                    >
                        {{ t('common.close') }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>
    </div>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted } from 'vue'
    import { useTranslations } from '@/composables/useTranslations'
    import { useErrorHandler } from '@/composables/useErrorHandler'
    import { typedHttpClient } from '@/api/typed-client'
    import type { SystemLog } from '@/api/clients/system-logs'

    const { t } = useTranslations()
    const { handleError } = useErrorHandler()

    const logs = ref<SystemLog[]>([])
    const total = ref(0)
    const loading = ref(false)
    const page = ref(1)
    const itemsPerPage = ref(25)

    const filterLogType = ref<string | null>(null)
    const filterResourceType = ref<string | null>(null)
    const filterStatus = ref<string | null>(null)

    const showEmailPreview = ref(false)
    const previewLog = ref<SystemLog | null>(null)

    const resourceRoutes: Record<string, string> = {
        admin_user: '/permissions',
        role: '/permissions',
        workflow: '/workflows',
        entity_definition: '/entity-definitions',
        email_template: '/system',
        email: '/system',
    }

    const logTypeOptions = [
        'email_sent',
        'entity_created',
        'entity_updated',
        'entity_deleted',
        'auth_event',
    ]

    const resourceTypeOptions = [
        'email',
        'admin_user',
        'role',
        'workflow',
        'entity_definition',
        'email_template',
    ]

    const statusOptions = ['success', 'failed', 'pending']

    const headers = computed(() => [
        { title: t('system.logs.timestamp'), key: 'created_at', sortable: false },
        { title: t('system.logs.type'), key: 'log_type', sortable: false },
        { title: t('system.logs.resource'), key: 'resource_type', sortable: false },
        { title: 'UUID', key: 'resource_uuid', sortable: false },
        { title: t('system.logs.summary'), key: 'summary', sortable: false },
        { title: t('system.logs.status'), key: 'status', sortable: false },
    ])

    const load = async () => {
        loading.value = true
        try {
            const result = await typedHttpClient.listSystemLogs({
                page: page.value,
                page_size: itemsPerPage.value,
                log_type: filterLogType.value ?? undefined,
                resource_type: filterResourceType.value ?? undefined,
                status: filterStatus.value ?? undefined,
            })
            logs.value = result.data
            total.value = result.total
        } catch (err) {
            handleError(err)
        } finally {
            loading.value = false
        }
    }

    const handleFilterChange = () => {
        page.value = 1
        void load()
    }

    const handlePageChange = (newPage: number) => {
        page.value = newPage
        void load()
    }

    const handleItemsPerPageChange = (newSize: number) => {
        itemsPerPage.value = newSize
        page.value = 1
        void load()
    }

    const openEmailPreview = (log: SystemLog) => {
        previewLog.value = log
        showEmailPreview.value = true
    }

    const getHtmlPreview = (details: unknown): string => {
        if (typeof details === 'object' && details !== null) {
            const d = details as Record<string, unknown>
            if (typeof d['body_html'] === 'string') return d['body_html']
            if (typeof d['html'] === 'string') return d['html']
        }
        return String(details ?? '')
    }

    const getLogTypeColor = (logType: string): string => {
        switch (logType) {
            case 'email_sent':
                return 'blue'
            case 'entity_created':
                return 'success'
            case 'entity_updated':
                return 'warning'
            case 'entity_deleted':
                return 'error'
            case 'auth_event':
                return 'purple'
            default:
                return 'grey'
        }
    }

    const getStatusColor = (status: string): string => {
        switch (status) {
            case 'success':
                return 'success'
            case 'failed':
                return 'error'
            case 'pending':
                return 'warning'
            default:
                return 'grey'
        }
    }

    const formatDate = (dateString: string): string => {
        try {
            return new Date(dateString).toLocaleString()
        } catch {
            return dateString
        }
    }

    onMounted(() => {
        void load()
    })
</script>

<style scoped>
    .email-preview-frame {
        border: 1px solid rgba(0, 0, 0, 0.12);
        border-radius: 4px;
        padding: 16px;
        background: white;
        max-height: 500px;
        overflow-y: auto;
    }
</style>
