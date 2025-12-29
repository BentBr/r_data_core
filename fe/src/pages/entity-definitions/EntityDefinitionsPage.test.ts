import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import EntityDefinitionsPage from './EntityDefinitionsPage.vue'

const mockGetEntityDefinitions = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getEntityDefinitions: (page?: number, itemsPerPage?: number) =>
            mockGetEntityDefinitions(page, itemsPerPage),
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
    routes: [{ path: '/entity-definitions', component: EntityDefinitionsPage }],
})

describe('EntityDefinitionsPage', () => {
    beforeEach(() => {
        vi.clearAllMocks()
        mockGetEntityDefinitions.mockResolvedValue({
            data: [
                {
                    entity_type: 'Customer',
                    display_name: 'Customer',
                    allow_children: true,
                    fields: [],
                },
            ],
        })
        // Default: user has create permission
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return (
                namespace === 'EntityDefinitions' &&
                (permission === 'Create' || permission === 'Admin')
            )
        })
    })

    it('shows create button when user has EntityDefinitions:Create permission', async () => {
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'EntityDefinitions' && permission === 'Create'
        })

        const wrapper = mount(EntityDefinitionsPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Check that canCreateEntityDefinition computed is true
        expect((wrapper.vm as any).canCreateEntityDefinition).toBe(true)
    })

    it('shows create button when user has EntityDefinitions:Admin permission', async () => {
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'EntityDefinitions' && permission === 'Admin'
        })

        const wrapper = mount(EntityDefinitionsPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Check that canCreateEntityDefinition computed is true
        expect((wrapper.vm as any).canCreateEntityDefinition).toBe(true)
    })

    it('hides create button when user lacks create permissions', async () => {
        mockHasPermission.mockImplementation(() => false)

        const wrapper = mount(EntityDefinitionsPage, {
            global: {
                plugins: [router],
            },
        })

        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 100))

        // Check that canCreateEntityDefinition computed is false
        expect((wrapper.vm as any).canCreateEntityDefinition).toBe(false)
    })
})
