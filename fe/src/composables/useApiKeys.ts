import { ref, computed } from 'vue'
import { typedHttpClient } from '@/api/typed-client'
import { useErrorHandler } from './useErrorHandler'
import { useTranslations } from './useTranslations'
import { useAuthStore } from '@/stores/auth'
import type { ApiKey, CreateApiKeyRequest } from '@/types/schemas'

export function useApiKeys() {
    const { handleError, handleSuccess } = useErrorHandler()
    const { t } = useTranslations()
    const authStore = useAuthStore()

    // State
    const apiKeys = ref<ApiKey[]>([])
    const loading = ref(false)
    const creating = ref(false)
    const revoking = ref(false)
    const error = ref('')
    const currentPage = ref(1)
    const itemsPerPage = ref(10)
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

    /**
     * Load API keys with pagination
     */
    const loadApiKeys = async (page = 1, perPage = 10): Promise<void> => {
        loading.value = true
        error.value = ''

        try {
            const response = await typedHttpClient.getApiKeys(page, perPage)
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
            currentPage.value = page
            itemsPerPage.value = perPage
        } catch (err) {
            console.error('Failed to load API keys:', err)
            error.value = err instanceof Error ? err.message : 'Failed to load API keys'
            handleError(err, 'Failed to load API keys')
        } finally {
            loading.value = false
        }
    }

    /**
     * Create an API key
     */
    const createApiKey = async (data: CreateApiKeyRequest): Promise<{ api_key: string } | null> => {
        creating.value = true

        try {
            const result = await typedHttpClient.createApiKey(data)

            // Reload the list
            await loadApiKeys(currentPage.value, itemsPerPage.value)

            handleSuccess(t('api_keys.create.success') || 'API key created successfully')
            return result
        } catch (err) {
            handleError(err, t('api_keys.create.error') || 'Failed to create API key')
            return null
        } finally {
            creating.value = false
        }
    }

    /**
     * Revoke an API key
     */
    const revokeApiKey = async (uuid: string): Promise<boolean> => {
        revoking.value = true

        try {
            await typedHttpClient.revokeApiKey(uuid)

            // Reload the list
            await loadApiKeys(currentPage.value, itemsPerPage.value)

            handleSuccess(t('api_keys.revoke.success') || 'API key revoked successfully')
            return true
        } catch (err) {
            handleError(err, t('api_keys.revoke.error') || 'Failed to revoke API key')
            return false
        } finally {
            revoking.value = false
        }
    }

    /**
     * Handle page change
     */
    const handlePageChange = async (page: number): Promise<void> => {
        currentPage.value = page
        await loadApiKeys(page, itemsPerPage.value)
    }

    /**
     * Handle items per page change
     */
    const handleItemsPerPageChange = async (perPage: number): Promise<void> => {
        itemsPerPage.value = perPage
        currentPage.value = 1
        await loadApiKeys(1, perPage)
    }

    /**
     * Computed: is user admin
     */
    const isAdmin = computed(() => {
        return authStore.user?.is_admin ?? false
    })

    /**
     * Format date for display
     */
    const formatDate = (dateString: string | null): string => {
        if (!dateString) {
            return 'Never'
        }
        return new Date(dateString).toLocaleString()
    }

    return {
        // State
        apiKeys,
        loading,
        creating,
        revoking,
        error,
        currentPage,
        itemsPerPage,
        totalItems,
        totalPages,
        paginationMeta,

        // Getters
        isAdmin,

        // Methods
        loadApiKeys,
        createApiKey,
        revokeApiKey,
        handlePageChange,
        handleItemsPerPageChange,
        formatDate,
    }
}
