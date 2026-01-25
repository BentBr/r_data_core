import { describe, it, expect, vi, beforeEach } from 'vitest'
import { useApiKeys } from './useApiKeys'
import type { ApiKey, CreateApiKeyRequest } from '@/types/schemas'

// Mock dependencies
const mockHandleError = vi.fn()
const mockHandleSuccess = vi.fn()
const mockT = vi.fn((key: string) => key)
const mockGetApiKeys = vi.fn()
const mockCreateApiKey = vi.fn()
const mockRevokeApiKey = vi.fn()
const mockUser = { is_admin: true }

vi.mock('./useErrorHandler', () => ({
    useErrorHandler: () => ({
        handleError: mockHandleError,
        handleSuccess: mockHandleSuccess,
    }),
}))

vi.mock('./useTranslations', () => ({
    useTranslations: () => ({
        t: mockT,
    }),
}))

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getApiKeys: (...args: unknown[]) => mockGetApiKeys(...args),
        createApiKey: (...args: unknown[]) => mockCreateApiKey(...args),
        revokeApiKey: (...args: unknown[]) => mockRevokeApiKey(...args),
    },
}))

vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        user: mockUser,
    }),
}))

describe('useApiKeys', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockGetApiKeys.mockResolvedValue({
            data: [],
            meta: { pagination: { total: 0, total_pages: 1, page: 1, per_page: 10 } },
        })
    })

    describe('initial state', () => {
        it('should initialize with empty state', () => {
            const { apiKeys, loading, creating, revoking, error } = useApiKeys()
            expect(apiKeys.value).toEqual([])
            expect(loading.value).toBe(false)
            expect(creating.value).toBe(false)
            expect(revoking.value).toBe(false)
            expect(error.value).toBe('')
        })

        it('should initialize pagination state', () => {
            const { currentPage, itemsPerPage, totalItems, totalPages } = useApiKeys()
            expect(currentPage.value).toBe(1)
            expect(itemsPerPage.value).toBe(10)
            expect(totalItems.value).toBe(0)
            expect(totalPages.value).toBe(1)
        })

        it('should compute isAdmin from auth store', () => {
            const { isAdmin } = useApiKeys()
            expect(isAdmin.value).toBe(true)
        })
    })

    describe('loadApiKeys', () => {
        it('should load API keys successfully', async () => {
            const mockApiKeys: ApiKey[] = [
                {
                    uuid: 'key-1',
                    name: 'Test Key',
                    is_active: true,
                    created_at: '2024-01-01T00:00:00Z',
                    expires_at: null,
                    last_used_at: null,
                    created_by: null,
                    user_uuid: 'user-1',
                    published: true,
                },
            ]
            mockGetApiKeys.mockResolvedValue({
                data: mockApiKeys,
                meta: { pagination: { total: 1, total_pages: 1, page: 1, per_page: 10 } },
            })

            const { loadApiKeys, apiKeys, loading } = useApiKeys()
            await loadApiKeys()

            expect(loading.value).toBe(false)
            expect(apiKeys.value).toEqual(mockApiKeys)
            expect(mockGetApiKeys).toHaveBeenCalled()
            expect(mockGetApiKeys.mock.calls[0][0]).toBe(1)
            expect(mockGetApiKeys.mock.calls[0][1]).toBe(10)
        })

        it('should handle loading errors', async () => {
            const error = new Error('Failed to load')
            mockGetApiKeys.mockRejectedValue(error)

            const { loadApiKeys, error: errorState, loading } = useApiKeys()
            await loadApiKeys()

            expect(loading.value).toBe(false)
            expect(errorState.value).toBe('Failed to load')
            expect(mockHandleError).toHaveBeenCalled()
        })

        it('should update pagination metadata', async () => {
            mockGetApiKeys.mockResolvedValue({
                data: [],
                meta: { pagination: { total: 50, total_pages: 5, page: 2, per_page: 10 } },
            })

            const { loadApiKeys, totalItems, totalPages, currentPage, itemsPerPage } = useApiKeys()
            await loadApiKeys(2, 10)

            expect(totalItems.value).toBe(50)
            expect(totalPages.value).toBe(5)
            expect(currentPage.value).toBe(2)
            expect(itemsPerPage.value).toBe(10)
        })
    })

    describe('createApiKey', () => {
        it('should create API key successfully', async () => {
            const request: CreateApiKeyRequest = { name: 'New Key' }
            const result = { api_key: 'test_key_12345' }
            mockCreateApiKey.mockResolvedValue(result)
            mockGetApiKeys.mockResolvedValue({
                data: [],
                meta: { pagination: { total: 0, total_pages: 1, page: 1, per_page: 10 } },
            })

            const { createApiKey, creating } = useApiKeys()
            const created = await createApiKey(request)

            expect(creating.value).toBe(false)
            expect(created).toEqual(result)
            expect(mockCreateApiKey).toHaveBeenCalledWith(request)
            expect(mockHandleSuccess).toHaveBeenCalled()
        })

        it('should handle creation errors', async () => {
            const request: CreateApiKeyRequest = { name: 'New Key' }
            const error = new Error('Creation failed')
            mockCreateApiKey.mockRejectedValue(error)

            const { createApiKey, creating } = useApiKeys()
            const result = await createApiKey(request)

            expect(creating.value).toBe(false)
            expect(result).toBeNull()
            expect(mockHandleError).toHaveBeenCalled()
        })
    })

    describe('revokeApiKey', () => {
        it('should revoke API key successfully', async () => {
            mockRevokeApiKey.mockResolvedValue(undefined)
            mockGetApiKeys.mockResolvedValue({
                data: [],
                meta: { pagination: { total: 0, total_pages: 1, page: 1, per_page: 10 } },
            })

            const { revokeApiKey, revoking } = useApiKeys()
            const result = await revokeApiKey('key-uuid')

            expect(revoking.value).toBe(false)
            expect(result).toBe(true)
            expect(mockRevokeApiKey).toHaveBeenCalledWith('key-uuid')
            expect(mockHandleSuccess).toHaveBeenCalled()
        })

        it('should handle revocation errors', async () => {
            const error = new Error('Revocation failed')
            mockRevokeApiKey.mockRejectedValue(error)

            const { revokeApiKey, revoking } = useApiKeys()
            const result = await revokeApiKey('key-uuid')

            expect(revoking.value).toBe(false)
            expect(result).toBe(false)
            expect(mockHandleError).toHaveBeenCalled()
        })
    })

    describe('formatDate', () => {
        it('should format date string', () => {
            const { formatDate } = useApiKeys()
            const dateStr = '2024-01-15T10:30:00Z'
            const formatted = formatDate(dateStr)
            expect(typeof formatted).toBe('string')
            expect(formatted.length).toBeGreaterThan(0)
        })

        it('should return "Never" for null date', () => {
            const { formatDate } = useApiKeys()
            expect(formatDate(null)).toBe('Never')
        })
    })

    describe('handlePageChange', () => {
        it('should change page and reload', async () => {
            mockGetApiKeys.mockResolvedValue({
                data: [],
                meta: { pagination: { total: 0, total_pages: 1, page: 2, per_page: 10 } },
            })

            const { handlePageChange, currentPage } = useApiKeys()
            await handlePageChange(2)

            expect(currentPage.value).toBe(2)
            expect(mockGetApiKeys).toHaveBeenCalled()
            expect(mockGetApiKeys.mock.calls[0][0]).toBe(2)
            expect(mockGetApiKeys.mock.calls[0][1]).toBe(10)
        })
    })

    describe('handleItemsPerPageChange', () => {
        it('should change items per page and reset to page 1', async () => {
            mockGetApiKeys.mockResolvedValue({
                data: [],
                meta: { pagination: { total: 0, total_pages: 1, page: 1, per_page: 25 } },
            })

            const { handleItemsPerPageChange, itemsPerPage, currentPage } = useApiKeys()
            await handleItemsPerPageChange(25)

            expect(itemsPerPage.value).toBe(25)
            expect(currentPage.value).toBe(1)
            expect(mockGetApiKeys).toHaveBeenCalled()
            const lastCall = mockGetApiKeys.mock.calls[mockGetApiKeys.mock.calls.length - 1]
            expect(lastCall[0]).toBe(1)
            expect(lastCall[1]).toBe(25)
        })
    })
})
