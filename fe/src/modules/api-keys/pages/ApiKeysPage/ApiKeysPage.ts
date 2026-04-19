import { ref, computed, onMounted, onUnmounted, nextTick, defineComponent } from 'vue'
import { useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { typedHttpClient } from '@/api/typed-client'
import { useTranslations } from '@/shared/composables/useTranslations'
import { useSnackbar } from '@/shared/composables/useSnackbar'
import { useErrorHandler } from '@/shared/composables/useErrorHandler'
import { usePagination } from '@/shared/composables/usePagination'
import type { ApiKey, CreateApiKeyRequest } from '@/types/schemas'
import ApiKeyCreateDialog from '@/modules/api-keys/components/ApiKeyCreateDialog/index.vue'
import ApiKeyViewDialog from '@/modules/api-keys/components/ApiKeyViewDialog/index.vue'
import ApiKeyCreatedDialog from '@/modules/api-keys/components/ApiKeyCreatedDialog/index.vue'
import DialogManager from '@/shared/components/DialogManager/index.vue'
import SnackbarManager from '@/shared/components/SnackbarManager/index.vue'
import PaginatedDataTable from '@/shared/tables/PaginatedDataTable/index.vue'
import PageLayout from '@/shared/components/PageLayout/index.vue'
import SmartIcon from '@/shared/components/SmartIcon/index.vue'
import Badge from '@/shared/components/Badge/index.vue'

export default defineComponent({
    name: 'ApiKeysPage',
    components: {
        ApiKeyCreateDialog,
        ApiKeyViewDialog,
        ApiKeyCreatedDialog,
        DialogManager,
        SnackbarManager,
        PaginatedDataTable,
        PageLayout,
        SmartIcon,
        Badge,
    },
    setup() {
        const authStore = useAuthStore()
        const route = useRoute()
        const { t } = useTranslations()
        const { currentSnackbar, showSuccess } = useSnackbar()
        const { handleError } = useErrorHandler()

        const canCreateApiKey = computed(() => {
            return (
                authStore.hasPermission('ApiKeys', 'Create') ||
                authStore.hasPermission('ApiKeys', 'Admin')
            )
        })

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

        const isComponentMounted = ref(false)

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

        const revokeDialogConfig = computed(() => ({
            title: t('api_keys.revoke.title'),
            confirmText: t('api_keys.revoke.button'),
            cancelText: t('common.cancel'),
            maxWidth: '400px',
        }))

        const loadApiKeys = async (page = 1, perPage = 10) => {
            if (!isComponentMounted.value || !authStore.isAuthenticated) {
                return
            }
            loading.value = true
            error.value = ''
            try {
                const response = await typedHttpClient.getApiKeys(
                    page,
                    perPage,
                    sortBy.value,
                    sortOrder.value
                )
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
            } finally {
                loading.value = false
            }
        }

        const handlePageChange = async (page: number) => {
            currentPage.value = page
            setPage(page)
            await loadApiKeys(currentPage.value, itemsPerPage.value)
        }

        const handleItemsPerPageChange = async (newItemsPerPage: number) => {
            itemsPerPage.value = newItemsPerPage
            setItemsPerPage(newItemsPerPage)
            currentPage.value = 1
            setPage(1)
            await loadApiKeys(1, newItemsPerPage)
        }

        const handleSortChange = async (
            newSortBy: string | null,
            newSortOrder: 'asc' | 'desc' | null
        ) => {
            sortBy.value = newSortBy
            sortOrder.value = newSortOrder
            currentPage.value = 1
            setPage(1)
            await loadApiKeys(1, itemsPerPage.value)
        }

        const createApiKey = async (requestData: CreateApiKeyRequest) => {
            creating.value = true
            try {
                const result = await typedHttpClient.createApiKey(requestData)
                createdApiKey.value = result.api_key
                showCreatedKeyDialog.value = true
                showCreateDialog.value = false
                await loadApiKeys()
            } catch (err) {
                handleError(err)
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
                await loadApiKeys()
            } catch (err) {
                handleError(err)
            } finally {
                revoking.value = false
            }
        }

        const handleCopySuccess = () => {
            showSuccess(t('api_keys.created.copied'))
        }

        const formatDate = (dateString: string | null): string => {
            if (!dateString) return 'Never'
            return new Date(dateString).toLocaleString()
        }

        onMounted(async () => {
            isComponentMounted.value = true
            await loadApiKeys(currentPage.value, itemsPerPage.value)
            if (route.query.create === 'true') {
                await nextTick()
                showCreateDialog.value = true
                window.history.replaceState({}, '', '/api-keys')
            }
        })

        onUnmounted(() => {
            isComponentMounted.value = false
        })

        return {
            t,
            loading,
            error,
            apiKeys,
            tableHeaders,
            currentPage,
            itemsPerPage,
            totalItems,
            totalPages,
            paginationMeta,
            canCreateApiKey,
            isAdmin,
            showCreateDialog,
            showViewDialog,
            showRevokeDialog,
            showCreatedKeyDialog,
            creating,
            revoking,
            selectedKey,
            keyToRevoke,
            createdApiKey,
            revokeDialogConfig,
            currentSnackbar,
            handlePageChange,
            handleItemsPerPageChange,
            handleSortChange,
            createApiKey,
            viewKey,
            confirmRevoke,
            revokeApiKey,
            handleCopySuccess,
            formatDate,
        }
    },
})
