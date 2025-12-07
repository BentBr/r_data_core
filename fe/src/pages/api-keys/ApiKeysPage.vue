<template>
    <v-container fluid>
        <v-row>
            <v-col cols="12">
                <v-card>
                    <v-card-title class="d-flex align-center justify-space-between pa-4">
                        <div class="d-flex align-center">
                            <SmartIcon
                                icon="key"
                                :size="28"
                                class="mr-3"
                            />
                            <span class="text-h4">API Keys Management</span>
                        </div>
                        <v-btn
                            color="primary"
                            @click="showCreateDialog = true"
                        >
                            <template #prepend>
                                <SmartIcon
                                    icon="key-round"
                                    :size="20"
                                />
                            </template>
                            {{ t('api_keys.create.button') }}
                        </v-btn>
                    </v-card-title>

                    <v-card-text>
                        <PaginatedDataTable
                            :items="apiKeys"
                            :headers="tableHeaders"
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
                            @update:sort="handleSortChange"
                        >
                            <!-- Name Column -->
                            <template #item.name="{ item }">
                                <div class="d-flex align-center">
                                    <SmartIcon
                                        :icon="item.is_active ? 'key' : 'key-round'"
                                        :color="item.is_active ? 'success' : 'error'"
                                        :size="20"
                                        class="mr-2"
                                    />
                                    <span
                                        :class="{ 'text-decoration-line-through': !item.is_active }"
                                    >
                                        {{ item.name }}
                                    </span>
                                </div>
                            </template>

                            <!-- Description Column -->
                            <template #item.description="{ item }">
                                <span class="text-body-2 text-medium-emphasis">
                                    {{ item.description ?? 'No description' }}
                                </span>
                            </template>

                            <!-- Status Column -->
                            <template #item.is_active="{ item }">
                                <Badge
                                    :status="item.is_active ? 'success' : 'error'"
                                    size="small"
                                >
                                    {{ item.is_active ? 'Active' : 'Inactive' }}
                                </Badge>
                            </template>

                            <!-- Created At Column -->
                            <template #item.created_at="{ item }">
                                <span class="text-body-2">
                                    {{ formatDate(item.created_at) }}
                                </span>
                            </template>

                            <!-- Expires At Column -->
                            <template #item.expires_at="{ item }">
                                <span
                                    v-if="item.expires_at"
                                    class="text-body-2"
                                >
                                    {{ formatDate(item.expires_at) }}
                                </span>
                                <span
                                    v-else
                                    class="text-body-2 text-medium-emphasis"
                                >
                                    Never
                                </span>
                            </template>

                            <!-- Last Used Column -->
                            <template #item.last_used_at="{ item }">
                                <span
                                    v-if="item.last_used_at"
                                    class="text-body-2"
                                >
                                    {{ formatDate(item.last_used_at) }}
                                </span>
                                <span
                                    v-else
                                    class="text-body-2 text-medium-emphasis"
                                >
                                    Never
                                </span>
                            </template>

                            <!-- User UUID Column (Admin Only) -->
                            <template #item.user_uuid="{ item }">
                                <span
                                    v-if="isAdmin"
                                    class="text-body-2 font-mono"
                                >
                                    {{ item.user_uuid }}
                                </span>
                                <span
                                    v-else
                                    class="text-body-2 text-medium-emphasis"
                                >
                                    -
                                </span>
                            </template>

                            <!-- Created By Column (Admin Only) -->
                            <template #item.created_by="{ item }">
                                <span
                                    v-if="isAdmin"
                                    class="text-body-2 font-mono"
                                >
                                    {{ item.created_by }}
                                </span>
                                <span
                                    v-else
                                    class="text-body-2 text-medium-emphasis"
                                >
                                    -
                                </span>
                            </template>

                            <!-- Actions Column -->
                            <template #item.actions="{ item }">
                                <div class="d-flex gap-2">
                                    <v-btn
                                        variant="text"
                                        size="small"
                                        color="info"
                                        :disabled="!item.is_active"
                                        @click="viewKey(item)"
                                    >
                                        <SmartIcon
                                            icon="eye"
                                            :size="20"
                                        />
                                    </v-btn>
                                    <v-btn
                                        variant="text"
                                        size="small"
                                        color="error"
                                        :disabled="!item.is_active"
                                        @click="confirmRevoke(item)"
                                    >
                                        <SmartIcon
                                            icon="trash-2"
                                            :size="20"
                                        />
                                    </v-btn>
                                </div>
                            </template>
                        </PaginatedDataTable>
                    </v-card-text>
                </v-card>
            </v-col>
        </v-row>

        <!-- Dialogs -->
        <ApiKeyCreateDialog
            v-model="showCreateDialog"
            :loading="creating"
            @create="createApiKey"
        />

        <ApiKeyViewDialog
            v-model="showViewDialog"
            :api-key="selectedKey"
        />

        <ApiKeyCreatedDialog
            v-model="showCreatedKeyDialog"
            :api-key="createdApiKey"
            @copy-success="handleCopySuccess"
        />

        <!-- Use DialogManager for revoke confirmation -->
        <DialogManager
            v-model="showRevokeDialog"
            :config="revokeDialogConfig"
            :loading="revoking"
            @confirm="revokeApiKey"
        >
            <p>{{ t('api_keys.revoke.message', { name: keyToRevoke?.name ?? 'Unknown' }) }}</p>
        </DialogManager>

        <!-- Snackbar -->
        <SnackbarManager :snackbar="currentSnackbar" />
    </v-container>
</template>

<script setup lang="ts">
    import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
    import { useRoute } from 'vue-router'
    import { useAuthStore } from '@/stores/auth'
    import { typedHttpClient } from '@/api/typed-client'
    import { useTranslations } from '@/composables/useTranslations'
    import { useSnackbar } from '@/composables/useSnackbar'
    import { usePagination } from '@/composables/usePagination'
    import type { ApiKey, CreateApiKeyRequest } from '@/types/schemas'
    import ApiKeyCreateDialog from '@/components/api-keys/ApiKeyCreateDialog.vue'
    import ApiKeyViewDialog from '@/components/api-keys/ApiKeyViewDialog.vue'
    import ApiKeyCreatedDialog from '@/components/api-keys/ApiKeyCreatedDialog.vue'
    import DialogManager from '@/components/common/DialogManager.vue'
    import SnackbarManager from '@/components/common/SnackbarManager.vue'
    import PaginatedDataTable from '@/components/tables/PaginatedDataTable.vue'
    import SmartIcon from '@/components/common/SmartIcon.vue'
import Badge from '@/components/common/Badge.vue'

    const authStore = useAuthStore()
    const route = useRoute()
    const { t } = useTranslations()
    const { currentSnackbar, showSuccess, showError } = useSnackbar()

    // Reactive state
    const loading = ref(false)
    const error = ref('')
    const apiKeys = ref<ApiKey[]>([])
    const showCreateDialog = ref(false)
    const showViewDialog = ref(false)
    const showRevokeDialog = ref(false)
    const showCreatedKeyDialog = ref(false)
    const creating = ref(false)
    const revoking = ref(false)
    const selectedKey = ref<ApiKey | null>(null)
    const keyToRevoke = ref<ApiKey | null>(null)
    const createdApiKey = ref('')
    const sortBy = ref<string | null>(null)
    const sortOrder = ref<'asc' | 'desc' | null>(null)

    // Pagination state with persistence
    const { state: paginationState, setPage, setItemsPerPage } = usePagination('api-keys', 10)
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

    // Component lifecycle flag
    const isComponentMounted = ref(false)

    // Computed properties
    const isAdmin = computed(() => {
        return authStore.user?.is_admin ?? false
    })

    const tableHeaders = computed(() => {
        const headers = [
            { title: t('api_keys.table.name'), key: 'name', sortable: true },
            { title: t('api_keys.table.description'), key: 'description', sortable: false },
            { title: t('api_keys.table.status'), key: 'is_active', sortable: true },
            { title: t('api_keys.table.created'), key: 'created_at', sortable: true },
            { title: t('api_keys.table.expires'), key: 'expires_at', sortable: true },
            { title: t('api_keys.table.last_used'), key: 'last_used_at', sortable: true },
        ]

        // Add admin-only columns
        if (isAdmin.value) {
            headers.splice(3, 0, {
                title: t('api_keys.table.user_id'),
                key: 'user_uuid',
                sortable: false,
            })
            headers.splice(4, 0, {
                title: t('api_keys.table.created_by'),
                key: 'created_by',
                sortable: false,
            })
        }

        headers.push({ title: t('api_keys.table.actions'), key: 'actions', sortable: false })

        return headers
    })

    // Dialog config for DialogManager
    const revokeDialogConfig = computed(() => ({
        title: t('api_keys.revoke.title'),
        confirmText: t('api_keys.revoke.button'),
        cancelText: t('common.cancel'),
        maxWidth: '400px',
    }))

    // Methods
    const loadApiKeys = async (page = 1, itemsPerPage = 10) => {
        // Don't load if component is not mounted or user is not authenticated
        if (!isComponentMounted.value || !authStore.isAuthenticated) {
            return
        }

        loading.value = true
        error.value = ''

        try {
            console.log(`Loading API keys: page=${page}, itemsPerPage=${itemsPerPage}, sortBy=${sortBy.value}, sortOrder=${sortOrder.value}`)
            const response = await typedHttpClient.getApiKeys(page, itemsPerPage, sortBy.value, sortOrder.value)
            apiKeys.value = response.data
            if (response.meta?.pagination) {
                totalItems.value = response.meta.pagination.total
                totalPages.value = response.meta.pagination.total_pages
                paginationMeta.value = response.meta.pagination
            } else {
                totalItems.value = apiKeys.value.length
                totalPages.value = 1
                paginationMeta.value = null
            }
        } catch (err) {
            console.error('Failed to load API keys:', err)
            error.value = err instanceof Error ? err.message : 'Failed to load API keys'
            // Don't clear items on error to maintain layout
        } finally {
            loading.value = false
        }
    }

    const handlePageChange = async (page: number) => {
        console.log('Page changed to:', page)
        currentPage.value = page
        setPage(page)
        await loadApiKeys(currentPage.value, itemsPerPage.value)
    }

    const handleItemsPerPageChange = async (newItemsPerPage: number) => {
        console.log('Items per page changed to:', newItemsPerPage)
        itemsPerPage.value = newItemsPerPage
        setItemsPerPage(newItemsPerPage)
        // Reset to first page when changing items per page
        currentPage.value = 1
        setPage(1)
        await loadApiKeys(1, newItemsPerPage)
    }

    const handleSortChange = async (newSortBy: string | null, newSortOrder: 'asc' | 'desc' | null) => {
        console.log('Sort changed:', newSortBy, newSortOrder)
        sortBy.value = newSortBy
        sortOrder.value = newSortOrder
        // Reset to first page when sorting changes
        currentPage.value = 1
        setPage(1)
        await loadApiKeys(1, itemsPerPage.value)
    }

    const createApiKey = async (requestData: CreateApiKeyRequest) => {
        creating.value = true

        try {
            const result = await typedHttpClient.createApiKey(requestData)

            // Show the API key in the dedicated dialog
            createdApiKey.value = result.api_key
            showCreatedKeyDialog.value = true

            // Close create dialog
            showCreateDialog.value = false

            // Reload the list
            await loadApiKeys()
        } catch (err) {
            // Handle specific error cases
            if (err instanceof Error) {
                if (err.message.includes('409') || err.message.includes('conflict')) {
                    showError(t('api_keys.create.error_name_exists'))
                } else {
                    showError(err.message ?? t('api_keys.create.error'))
                }
            } else {
                showError(t('api_keys.create.error'))
            }
        } finally {
            creating.value = false
        }
    }

    const viewKey = (key: ApiKey) => {
        selectedKey.value = key
        showViewDialog.value = true
    }

    const confirmRevoke = (key: ApiKey) => {
        keyToRevoke.value = key
        showRevokeDialog.value = true
    }

    const revokeApiKey = async () => {
        if (!keyToRevoke.value) {
            return
        }

        revoking.value = true

        try {
            await typedHttpClient.revokeApiKey(keyToRevoke.value.uuid)

            showSuccess(t('api_keys.revoke.success'))

            showRevokeDialog.value = false
            keyToRevoke.value = null

            // Reload the list
            await loadApiKeys()
        } catch (err) {
            showError(err instanceof Error ? err.message : t('api_keys.revoke.error'))
        } finally {
            revoking.value = false
        }
    }

    const handleCopySuccess = () => {
        showSuccess(t('api_keys.created.copied'))
    }

    const formatDate = (dateString: string | null): string => {
        if (!dateString) {
            return 'Never'
        }
        return new Date(dateString).toLocaleString()
    }

    // Lifecycle
    onMounted(async () => {
        isComponentMounted.value = true
        await loadApiKeys(currentPage.value, itemsPerPage.value)

        // Check for query params to open dialogs
        if (route.query.create === 'true') {
            await nextTick()
            showCreateDialog.value = true
            // Remove query param from URL
            window.history.replaceState({}, '', '/api-keys')
        }
    })

    onUnmounted(() => {
        isComponentMounted.value = false
    })
</script>

<style scoped>
    /* Ensure stable layout to prevent scrollbar shifts */
    .v-container {
        min-height: calc(100vh - 64px - 32px); /* Account for app bar and padding */
        overflow-x: hidden;
    }

    /* Ensure table container has stable dimensions */
    .v-card {
        min-height: 400px;
    }
</style>
