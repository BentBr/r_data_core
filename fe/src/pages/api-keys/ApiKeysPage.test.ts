import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import ApiKeysPage from './ApiKeysPage.vue'

const mockGetApiKeys = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getApiKeys: (page?: number, itemsPerPage?: number) => mockGetApiKeys(page, itemsPerPage),
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

const mockHasPermission = vi.fn()
vi.mock('@/stores/auth', () => ({
    useAuthStore: () => ({
        hasPermission: mockHasPermission,
    }),
}))

const router = createRouter({
    history: createWebHistory(),
    routes: [{ path: '/api-keys', component: ApiKeysPage }],
})

describe('ApiKeysPage', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockGetApiKeys.mockResolvedValue({
            data: [],
            meta: { pagination: { total: 0, total_pages: 1, page: 1, per_page: 20 } },
        })
        // Default: user has create permission
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'ApiKeys' && (permission === 'Create' || permission === 'Admin')
        })
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
        await new Promise(resolve => setTimeout(resolve, 100))

        // Check that canCreateApiKey computed is true
        expect((wrapper.vm as any).canCreateApiKey).toBe(true)
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
        await new Promise(resolve => setTimeout(resolve, 100))

        // Check that canCreateApiKey computed is true
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
        await new Promise(resolve => setTimeout(resolve, 100))

        // Check that canCreateApiKey computed is false
        expect((wrapper.vm as any).canCreateApiKey).toBe(false)
    })
})
