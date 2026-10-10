import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import ApiKeysPage from './ApiKeysPage.vue'
import type { ApiKey } from '@/types/schemas'

const mockGetApiKeys: Mock = vi.fn()
const mockCreateApiKey: Mock = vi.fn()
const mockRevokeApiKey: Mock = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getApiKeys: (
            page?: number,
            itemsPerPage?: number,
            sortBy?: string | null,
            sortOrder?: string | null
        ) => mockGetApiKeys(page, itemsPerPage, sortBy, sortOrder),
        createApiKey: (data: unknown) => mockCreateApiKey(data),
        revokeApiKey: (uuid: string) => mockRevokeApiKey(uuid),
    },
}))

vi.mock('@/composables/useTranslations', () => ({
    useTranslations: () => ({ t: (k: string) => k.split('.').pop() }),
}))

const showSuccess = vi.fn()
const showError = vi.fn()
vi.mock('@/composables/useSnackbar', () => ({
    useSnackbar: () => ({
        currentSnackbar: null,
        showSuccess,
        showError,
    }),
}))

const mockHandleError = vi.fn()
vi.mock('@/composables/useErrorHandler', () => ({
    useErrorHandler: () => ({
        handleError: mockHandleError,
    }),
}))

const mockHasPermission = vi.fn()
let mockUser: { is_admin: boolean } | null = { is_admin: false }
let mockIsAuthenticated = true
vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        hasPermission: mockHasPermission,
        get user() {
            return mockUser
        },
        get isAuthenticated() {
            return mockIsAuthenticated
        },
    }),
}))

const router = createRouter({
    history: createWebHistory(),
    routes: [{ path: '/api-keys', component: ApiKeysPage }],
})

const makeApiKey = (overrides: Partial<ApiKey> = {}): ApiKey => ({
    uuid: 'key-1',
    name: 'My Key',
    description: 'A description',
    is_active: true,
    created_at: '2024-01-01T00:00:00Z',
    expires_at: null,
    last_used_at: null,
    created_by: 'user-1',
    user_uuid: 'user-1',
    published: true,
    ...overrides,
})

const flush = async () => {
    await Promise.resolve()
    await Promise.resolve()
}

describe('ApiKeysPage', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        localStorage.clear()
        mockUser = { is_admin: false }
        mockIsAuthenticated = true
        mockGetApiKeys.mockResolvedValue({
            data: [],
            meta: { pagination: { total: 0, total_pages: 1, page: 1, per_page: 20 } },
        })
        mockCreateApiKey.mockResolvedValue({ api_key: 'secret-value' })
        mockRevokeApiKey.mockResolvedValue(undefined)
        // Default: user has create permission
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'ApiKeys' && (permission === 'Create' || permission === 'Admin')
        })
        if (router.currentRoute.value.path !== '/api-keys') {
            void router.push('/api-keys')
        }
    })

    it('shows create button when user has ApiKeys:Create permission', async () => {
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'ApiKeys' && permission === 'Create'
        })

        const wrapper = mount(ApiKeysPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await flush()

        expect((wrapper.vm as any).canCreateApiKey).toBe(true)
        expect(wrapper.find('[data-testid="api-keys-create-btn"]').exists()).toBe(true)
    })

    it('shows create button when user has ApiKeys:Admin permission', async () => {
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'ApiKeys' && permission === 'Admin'
        })

        const wrapper = mount(ApiKeysPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await flush()

        expect((wrapper.vm as any).canCreateApiKey).toBe(true)
    })

    it('hides create button when user lacks create permissions', async () => {
        mockHasPermission.mockImplementation(() => false)

        const wrapper = mount(ApiKeysPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await flush()

        expect((wrapper.vm as any).canCreateApiKey).toBe(false)
        expect(wrapper.find('[data-testid="api-keys-create-btn"]').exists()).toBe(false)
    })

    it('loads API keys on mount and populates pagination metadata', async () => {
        const key = makeApiKey()
        mockGetApiKeys.mockResolvedValue({
            data: [key],
            meta: {
                pagination: {
                    total: 1,
                    total_pages: 1,
                    page: 1,
                    per_page: 10,
                    has_next: false,
                    has_previous: false,
                },
            },
        })

        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()

        expect(mockGetApiKeys).toHaveBeenCalledWith(1, 10, null, null)
        expect((wrapper.vm as any).apiKeys).toEqual([key])
        expect((wrapper.vm as any).totalItems).toBe(1)
        expect((wrapper.vm as any).totalPages).toBe(1)
    })

    it('does not load API keys when the user is not authenticated', async () => {
        mockIsAuthenticated = false

        mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()

        expect(mockGetApiKeys).not.toHaveBeenCalled()
    })

    it('falls back to item length when response has no pagination meta', async () => {
        const key = makeApiKey()
        mockGetApiKeys.mockResolvedValue({ data: [key], meta: {} })

        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()

        expect((wrapper.vm as any).totalItems).toBe(1)
        expect((wrapper.vm as any).totalPages).toBe(1)
        expect((wrapper.vm as any).paginationMeta).toBeNull()
    })

    it('sets an error message when loading fails', async () => {
        mockGetApiKeys.mockRejectedValue(new Error('network down'))

        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()

        expect((wrapper.vm as any).error).toBe('network down')
        expect((wrapper.vm as any).loading).toBe(false)
    })

    it('sets a generic error message when a non-Error is thrown', async () => {
        mockGetApiKeys.mockRejectedValue('boom')

        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()

        expect((wrapper.vm as any).error).toBe('Failed to load API keys')
    })

    it('shows admin-only columns when the user is an admin', async () => {
        mockUser = { is_admin: true }

        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()

        const keys = (wrapper.vm as any).tableHeaders.map((h: { key: string }) => h.key)
        expect(keys).toContain('user_uuid')
        expect(keys).toContain('created_by')
    })

    it('omits admin-only columns for non-admin users', async () => {
        mockUser = { is_admin: false }

        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()

        const keys = (wrapper.vm as any).tableHeaders.map((h: { key: string }) => h.key)
        expect(keys).not.toContain('user_uuid')
        expect(keys).not.toContain('created_by')
    })

    it('handles page change by updating state and reloading', async () => {
        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()
        mockGetApiKeys.mockClear()

        await (wrapper.vm as any).handlePageChange(3)

        expect((wrapper.vm as any).currentPage).toBe(3)
        expect(mockGetApiKeys).toHaveBeenCalledWith(3, 10, null, null)
    })

    it('handles items-per-page change by resetting to page 1', async () => {
        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()
        await (wrapper.vm as any).handlePageChange(2)
        mockGetApiKeys.mockClear()

        await (wrapper.vm as any).handleItemsPerPageChange(25)

        expect((wrapper.vm as any).itemsPerPage).toBe(25)
        expect((wrapper.vm as any).currentPage).toBe(1)
        expect(mockGetApiKeys).toHaveBeenCalledWith(1, 25, null, null)
    })

    it('handles sort change by resetting to page 1 and passing sort params', async () => {
        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()
        await (wrapper.vm as any).handlePageChange(4)
        mockGetApiKeys.mockClear()

        await (wrapper.vm as any).handleSortChange('name', 'asc')

        expect((wrapper.vm as any).currentPage).toBe(1)
        expect(mockGetApiKeys).toHaveBeenCalledWith(1, 10, 'name', 'asc')
    })

    it('creates an API key, shows the created-key dialog, and reloads the list', async () => {
        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()
        ;(wrapper.vm as any).showCreateDialog = true
        mockGetApiKeys.mockClear()

        await (wrapper.vm as any).createApiKey({ name: 'New key' })

        expect(mockCreateApiKey).toHaveBeenCalledWith({ name: 'New key' })
        expect((wrapper.vm as any).createdApiKey).toBe('secret-value')
        expect((wrapper.vm as any).showCreatedKeyDialog).toBe(true)
        expect((wrapper.vm as any).showCreateDialog).toBe(false)
        expect((wrapper.vm as any).creating).toBe(false)
        expect(mockGetApiKeys).toHaveBeenCalled()
    })

    it('routes create errors through the error handler', async () => {
        mockCreateApiKey.mockRejectedValue(new Error('create failed'))
        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()

        await (wrapper.vm as any).createApiKey({ name: 'bad' })

        expect(mockHandleError).toHaveBeenCalled()
        expect((wrapper.vm as any).creating).toBe(false)
    })

    it('opens the view dialog with the selected key', async () => {
        const key = makeApiKey()
        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()

        ;(wrapper.vm as any).viewKey(key)

        expect((wrapper.vm as any).selectedKey).toEqual(key)
        expect((wrapper.vm as any).showViewDialog).toBe(true)
    })

    it('opens the revoke confirmation dialog for a key', async () => {
        const key = makeApiKey()
        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()

        ;(wrapper.vm as any).confirmRevoke(key)

        expect((wrapper.vm as any).keyToRevoke).toEqual(key)
        expect((wrapper.vm as any).showRevokeDialog).toBe(true)
    })

    it('revokes the selected key, shows success, and reloads the list', async () => {
        const key = makeApiKey()
        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()
        ;(wrapper.vm as any).confirmRevoke(key)
        mockGetApiKeys.mockClear()

        await (wrapper.vm as any).revokeApiKey()

        expect(mockRevokeApiKey).toHaveBeenCalledWith('key-1')
        expect(showSuccess).toHaveBeenCalled()
        expect((wrapper.vm as any).showRevokeDialog).toBe(false)
        expect((wrapper.vm as any).keyToRevoke).toBeNull()
        expect(mockGetApiKeys).toHaveBeenCalled()
    })

    it('does nothing when revoking without a selected key', async () => {
        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()

        await (wrapper.vm as any).revokeApiKey()

        expect(mockRevokeApiKey).not.toHaveBeenCalled()
    })

    it('routes revoke errors through the error handler', async () => {
        mockRevokeApiKey.mockRejectedValue(new Error('revoke failed'))
        const key = makeApiKey()
        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()
        ;(wrapper.vm as any).confirmRevoke(key)

        await (wrapper.vm as any).revokeApiKey()

        expect(mockHandleError).toHaveBeenCalled()
        expect((wrapper.vm as any).revoking).toBe(false)
    })

    it('shows a success message when the key is copied', async () => {
        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()

        ;(wrapper.vm as any).handleCopySuccess()

        expect(showSuccess).toHaveBeenCalled()
    })

    it('formats dates and falls back to "Never" for null values', async () => {
        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()

        expect((wrapper.vm as any).formatDate(null)).toBe('Never')
        expect((wrapper.vm as any).formatDate('2024-01-01T00:00:00Z')).not.toBe('Never')
    })

    it('opens the create dialog automatically when the route has create=true', async () => {
        const replaceStateSpy = vi
            .spyOn(window.history, 'replaceState')
            .mockImplementation(() => {})
        await router.push('/api-keys?create=true')

        const wrapper = mount(ApiKeysPage, { global: { plugins: [router] } })
        await flush()
        await wrapper.vm.$nextTick()

        expect((wrapper.vm as any).showCreateDialog).toBe(true)
        expect(replaceStateSpy).toHaveBeenCalledWith({}, '', '/api-keys')

        replaceStateSpy.mockRestore()
        await router.push('/api-keys')
    })
})
