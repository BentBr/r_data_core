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
            <v-text-field
                v-model="filterResourceUuid"
                label="Resource UUID"
                clearable
                density="compact"
                variant="outlined"
                style="min-width: 280px; max-width: 340px"
                prepend-inner-icon="search"
                @update:model-value="handleResourceUuidSearch"
            />
            <v-text-field
                v-model="filterDateFrom"
                :label="t('system.logs.filter_date_from')"
                type="datetime-local"
                clearable
                density="compact"
                variant="outlined"
                style="min-width: 220px; max-width: 260px"
                @update:model-value="handleFilterChange"
            />
            <v-text-field
                v-model="filterDateTo"
                :label="t('system.logs.filter_date_to')"
                type="datetime-local"
                clearable
                density="compact"
                variant="outlined"
                style="min-width: 220px; max-width: 260px"
                @update:model-value="handleFilterChange"
            />
        </div>

        <v-data-table-server
            :items="logs"
            :headers="headers"
            :loading="loading"
            :items-length="totalItems"
            :items-per-page="itemsPerPage"
            :page="page"
            :no-data-text="t('system.logs.no_logs')"
            data-testid="system-logs-table"
            hover
            @update:page="handlePageChange"
            @update:items-per-page="handleItemsPerPageChange"
            @click:row="onRowClick"
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
                    v-if="
                        item.resource_uuid &&
                        getResourceRoute(item.resource_type, item.resource_uuid)
                    "
                    :to="getResourceRoute(item.resource_type, item.resource_uuid)!"
                    class="text-body-2 text-decoration-none"
                >
                    <code>{{ item.resource_uuid.slice(0, 8) }}…</code>
                </router-link>
                <span
                    v-else-if="item.resource_uuid"
                    class="text-body-2 text-medium-emphasis"
                >
                    <code>{{ item.resource_uuid.slice(0, 8) }}…</code>
                </span>
                <span
                    v-else
                    class="text-medium-emphasis"
                >
                    —
                </span>
            </template>

            <template #item.summary="{ item }">
                <span class="text-body-2">{{ item.summary }}</span>
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

        <!-- Log detail dialog -->
        <v-dialog
            v-model="showDetail"
            max-width="800px"
        >
            <v-card v-if="detailLog">
                <v-card-title class="pa-6">
                    {{ t('system.logs.details') }}
                </v-card-title>
                <v-card-text class="pa-6">
                    <v-table density="compact">
                        <tbody>
                            <tr>
                                <td
                                    class="font-weight-bold"
                                    style="width: 160px"
                                >
                                    UUID
                                </td>
                                <td>
                                    <code>{{ detailLog.uuid }}</code>
                                </td>
                            </tr>
                            <tr>
                                <td class="font-weight-bold">{{ t('system.logs.timestamp') }}</td>
                                <td>{{ formatDate(detailLog.created_at) }}</td>
                            </tr>
                            <tr>
                                <td class="font-weight-bold">{{ t('system.logs.type') }}</td>
                                <td>
                                    <v-chip
                                        :color="getLogTypeColor(detailLog.log_type)"
                                        size="small"
                                        variant="tonal"
                                    >
                                        {{ detailLog.log_type }}
                                    </v-chip>
                                </td>
                            </tr>
                            <tr>
                                <td class="font-weight-bold">{{ t('system.logs.resource') }}</td>
                                <td>{{ detailLog.resource_type }}</td>
                            </tr>
                            <tr>
                                <td class="font-weight-bold">Resource UUID</td>
                                <td>
                                    <router-link
                                        v-if="
                                            detailLog.resource_uuid &&
                                            getResourceRoute(
                                                detailLog.resource_type,
                                                detailLog.resource_uuid
                                            )
                                        "
                                        :to="
                                            getResourceRoute(
                                                detailLog.resource_type,
                                                detailLog.resource_uuid
                                            )!
                                        "
                                    >
                                        <code>{{ detailLog.resource_uuid }}</code>
                                    </router-link>
                                    <code v-else-if="detailLog.resource_uuid">{{
                                        detailLog.resource_uuid
                                    }}</code>
                                    <span
                                        v-else
                                        class="text-medium-emphasis"
                                        >—</span
                                    >
                                </td>
                            </tr>
                            <tr>
                                <td class="font-weight-bold">{{ t('system.logs.actor') }}</td>
                                <td>
                                    <code v-if="detailLog.created_by">{{
                                        detailLog.created_by
                                    }}</code>
                                    <span
                                        v-else
                                        class="text-medium-emphasis"
                                        >—</span
                                    >
                                </td>
                            </tr>
                            <tr>
                                <td class="font-weight-bold">{{ t('system.logs.summary') }}</td>
                                <td>{{ detailLog.summary }}</td>
                            </tr>
                            <tr>
                                <td class="font-weight-bold">{{ t('system.logs.status') }}</td>
                                <td>
                                    <v-chip
                                        :color="getStatusColor(detailLog.status)"
                                        size="small"
                                        variant="tonal"
                                    >
                                        {{ detailLog.status }}
                                    </v-chip>
                                </td>
                            </tr>
                        </tbody>
                    </v-table>

                    <!-- Details JSON -->
                    <div
                        v-if="detailLog.details"
                        class="mt-4"
                    >
                        <h4 class="text-subtitle-2 mb-2">{{ t('system.logs.details') }}</h4>

                        <!-- Email HTML preview -->
                        <div
                            v-if="
                                detailLog.log_type === 'email_sent' &&
                                getHtmlPreview(detailLog.details)
                            "
                            class="email-preview-frame mb-4"
                            v-html="getHtmlPreview(detailLog.details)"
                        />

                        <!-- Raw JSON -->
                        <pre class="json-details pa-3 rounded">{{
                            JSON.stringify(detailLog.details, null, 2)
                        }}</pre>
                    </div>
                </v-card-text>
                <v-card-actions class="pa-4 px-6">
                    <v-spacer />
                    <v-btn
                        variant="text"
                        @click="showDetail = false"
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
    import type { SystemLogType } from '@/types/generated/SystemLogType'
    import type { SystemLogResourceType } from '@/types/generated/SystemLogResourceType'
    import type { SystemLogStatus } from '@/types/generated/SystemLogStatus'

    const { t } = useTranslations()
    const { handleError } = useErrorHandler()

    const logs = ref<SystemLog[]>([])
    const totalItems = ref(0)
    const loading = ref(false)
    const page = ref(1)
    const itemsPerPage = ref(25)

    const filterLogType = ref<SystemLogType | null>(null)
    const filterResourceType = ref<SystemLogResourceType | null>(null)
    const filterStatus = ref<SystemLogStatus | null>(null)
    const filterResourceUuid = ref<string | null>(null)
    const filterDateFrom = ref<string | null>(null)
    const filterDateTo = ref<string | null>(null)

    let resourceUuidDebounce: ReturnType<typeof setTimeout> | null = null

    const showDetail = ref(false)
    const detailLog = ref<SystemLog | null>(null)

    const resourceRouteTemplates: Record<string, string> = {
        admin_user: '/permissions/users/{uuid}',
        role: '/permissions/roles/{uuid}',
        workflow: '/workflows/{uuid}',
        entity_definition: '/entity-definitions/{uuid}',
        email_template: '/system',
        api_key: '/api-keys',
        system_settings: '/system',
        email: '/system',
    }

    const getResourceRoute = (resourceType: string, resourceUuid: string): string | null => {
        const template = resourceRouteTemplates[resourceType]
        if (!template) return null
        return template.replace('{uuid}', resourceUuid)
    }

    const logTypeOptions: SystemLogType[] = [
        'email_sent',
        'entity_created',
        'entity_updated',
        'entity_deleted',
        'auth_event',
    ]

    const resourceTypeOptions: SystemLogResourceType[] = [
        'email',
        'admin_user',
        'role',
        'workflow',
        'entity_definition',
        'email_template',
        'api_key',
        'system_settings',
    ]

    const statusOptions: SystemLogStatus[] = ['success', 'failed', 'pending']

    const headers = computed(() => [
        { title: t('system.logs.timestamp'), key: 'created_at', sortable: false },
        { title: t('system.logs.type'), key: 'log_type', sortable: false },
        { title: t('system.logs.resource'), key: 'resource_type', sortable: false },
        { title: 'Resource UUID', key: 'resource_uuid', sortable: false },
        { title: t('system.logs.summary'), key: 'summary', sortable: false },
        { title: t('system.logs.status'), key: 'status', sortable: false },
    ])

    const toIso8601 = (dateStr: string | null): string | undefined => {
        if (!dateStr) return undefined
        // datetime-local gives local time (e.g. "2026-04-19T13:04")
        // Convert to UTC via Date object so the filter matches the DB (which stores UTC)
        const date = new Date(dateStr)
        if (isNaN(date.getTime())) return undefined
        return date.toISOString()
    }

    const load = async () => {
        loading.value = true
        try {
            const result = await typedHttpClient.listSystemLogs({
                page: page.value,
                page_size: itemsPerPage.value,
                log_type: filterLogType.value ?? undefined,
                resource_type: filterResourceType.value ?? undefined,
                status: filterStatus.value ?? undefined,
                resource_uuid: filterResourceUuid.value ?? undefined,
                date_from: toIso8601(filterDateFrom.value),
                date_to: toIso8601(filterDateTo.value),
            })
            logs.value = result.data
            totalItems.value = result.meta?.pagination?.total ?? 0
        } catch (err) {
            handleError(err)
            logs.value = []
            totalItems.value = 0
        } finally {
            loading.value = false
        }
    }

    const handleFilterChange = () => {
        page.value = 1
        void load()
    }

    const handleResourceUuidSearch = () => {
        if (resourceUuidDebounce) clearTimeout(resourceUuidDebounce)
        resourceUuidDebounce = setTimeout(() => {
            page.value = 1
            void load()
        }, 400)
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

    const openDetailView = (log: SystemLog) => {
        detailLog.value = log
        showDetail.value = true
    }

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const onRowClick = (_event: any, row: any) => {
        openDetailView(row.item as SystemLog)
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
        color: #333;
        max-height: 400px;
        overflow-y: auto;
    }

    .json-details {
        background: rgba(0, 0, 0, 0.05);
        font-family: monospace;
        font-size: 0.85rem;
        white-space: pre-wrap;
        word-break: break-word;
        max-height: 300px;
        overflow-y: auto;
    }
</style>
