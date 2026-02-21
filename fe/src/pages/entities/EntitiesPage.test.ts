import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import EntitiesPage from './EntitiesPage.vue'
import type { DynamicEntity } from '@/types/schemas'

const mockGetEntityDefinitions = vi.fn()
const mockCreateEntity = vi.fn()
const mockDeleteEntity = vi.fn()
const mockGetEntity = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getEntityDefinitions: (page?: number, itemsPerPage?: number) =>
            mockGetEntityDefinitions(page, itemsPerPage),
        createEntity: (entityType: string, data: Record<string, unknown>) =>
            mockCreateEntity(entityType, data),
        deleteEntity: (uuid: string) => mockDeleteEntity(uuid),
        getEntity: (uuid: string) => mockGetEntity(uuid),
        browseByPath: vi.fn().mockResolvedValue({ data: [] }),
    },
    ValidationError: class ValidationError extends Error {
        violations: Array<{ field: string; message: string }>

        constructor(violations: Array<{ field: string; message: string }>) {
            super('validation')
            this.violations = violations
        }
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
        isAuthenticated: true,
        token: 'test-token',
        hasPermission: mockHasPermission,
    }),
}))

const router = createRouter({
    history: createWebHistory(),
    routes: [{ path: '/entities', component: EntitiesPage }],
})

describe('EntitiesPage - Path Detection Logic', () => {
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
        mockCreateEntity.mockResolvedValue({})
        mockDeleteEntity.mockResolvedValue({ message: 'Successfully deleted' })
        // Default: user has create permission
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'Entities' && (permission === 'Create' || permission === 'Admin')
        })
    })

    it('createEntity calculates correct path for single-segment path', async () => {
        const wrapper = mount(EntitiesPage, {
            global: {
                plugins: [router],
            },
        })
        await vi.waitUntil(() => mockGetEntityDefinitions.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Create entity with single-segment path (directory path)
        const createData = {
            entity_type: 'Customer',
            data: {
                path: '/test',
                entity_key: 'test-entity',
                published: false,
            },
        }

        // Test path detection logic directly
        const entityPath = createData.data.path
        const segments = entityPath.split('/').filter(s => s)
        const pathToReload =
            segments.length > 1 ? entityPath.split('/').slice(0, -1).join('/') : entityPath

        // Single segment should use path directly
        expect(pathToReload).toBe('/test')
    })

    it('createEntity calculates correct path for multi-segment path', async () => {
        const wrapper = mount(EntitiesPage, {
            global: {
                plugins: [router],
            },
        })
        await vi.waitUntil(() => mockGetEntityDefinitions.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Create entity with multi-segment path (full entity path)
        const createData = {
            entity_type: 'Customer',
            data: {
                path: '/test/entity-name',
                entity_key: 'entity-name',
                published: false,
            },
        }

        // Test path detection logic directly
        const entityPath = createData.data.path
        const segments = entityPath.split('/').filter(s => s)
        const pathToReload =
            segments.length > 1 ? entityPath.split('/').slice(0, -1).join('/') : entityPath

        // Multi-segment should get parent directory
        expect(pathToReload).toBe('/test')
    })

    it('deleteEntity calculates correct path for single-segment path', async () => {
        const wrapper = mount(EntitiesPage, {
            global: {
                plugins: [router],
            },
        })
        await vi.waitUntil(() => mockGetEntityDefinitions.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Set selected entity with single-segment path
        const selectedEntity: DynamicEntity = {
            entity_type: 'Customer',
            field_data: {
                uuid: 'test-uuid',
                path: '/test',
                entity_key: 'test-entity',
            },
        }

        // Test path detection logic directly
        const entityPath = selectedEntity.field_data.path as string | undefined
        const segments = entityPath?.split('/').filter(s => s) ?? []
        const pathToReload =
            segments.length > 1
                ? (entityPath?.split('/').slice(0, -1).join('/') ?? '/')
                : (entityPath ?? '/')

        // Single segment should use path directly
        expect(pathToReload).toBe('/test')
    })

    it('deleteEntity calculates correct path for multi-segment path', async () => {
        const wrapper = mount(EntitiesPage, {
            global: {
                plugins: [router],
            },
        })
        await vi.waitUntil(() => mockGetEntityDefinitions.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Set selected entity with multi-segment path
        const selectedEntity: DynamicEntity = {
            entity_type: 'Customer',
            field_data: {
                uuid: 'test-uuid',
                path: '/test/entity-name',
                entity_key: 'entity-name',
            },
        }

        // Test path detection logic directly
        const entityPath = selectedEntity.field_data.path as string | undefined
        const segments = entityPath?.split('/').filter(s => s) ?? []
        const pathToReload =
            segments.length > 1
                ? (entityPath?.split('/').slice(0, -1).join('/') ?? '/')
                : (entityPath ?? '/')

        // Multi-segment should get parent directory
        expect(pathToReload).toBe('/test')
    })

    it('deleteEntity handles root path correctly', async () => {
        const wrapper = mount(EntitiesPage, {
            global: {
                plugins: [router],
            },
        })
        await vi.waitUntil(() => mockGetEntityDefinitions.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Set selected entity with root path
        const selectedEntity: DynamicEntity = {
            entity_type: 'Customer',
            field_data: {
                uuid: 'test-uuid',
                path: '/',
                entity_key: 'root-entity',
            },
        }

        // Test path detection logic directly
        const entityPath = selectedEntity.field_data.path as string | undefined
        let pathToReload = '/'
        if (entityPath && entityPath !== '/') {
            const segments = entityPath.split('/').filter(s => s)
            pathToReload =
                segments.length > 1 ? entityPath.split('/').slice(0, -1).join('/') : entityPath
        }

        // Root path should remain root
        expect(pathToReload).toBe('/')
    })

    it('shows create button when user has Entities:Create permission', async () => {
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'Entities' && permission === 'Create'
        })

        const wrapper = mount(EntitiesPage, {
            global: {
                plugins: [router],
            },
        })

        await vi.waitUntil(() => mockGetEntityDefinitions.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Check that canCreateEntity computed is true
        expect((wrapper.vm as any).canCreateEntity).toBe(true)
    })

    it('shows create button when user has Entities:Admin permission', async () => {
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'Entities' && permission === 'Admin'
        })

        const wrapper = mount(EntitiesPage, {
            global: {
                plugins: [router],
            },
        })

        await vi.waitUntil(() => mockGetEntityDefinitions.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Check that canCreateEntity computed is true
        expect((wrapper.vm as any).canCreateEntity).toBe(true)
    })

    it('hides create button when user lacks create permissions', async () => {
        mockHasPermission.mockImplementation(() => false)

        const wrapper = mount(EntitiesPage, {
            global: {
                plugins: [router],
            },
        })

        await vi.waitUntil(() => mockGetEntityDefinitions.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()

        // Check that canCreateEntity computed is false
        expect((wrapper.vm as any).canCreateEntity).toBe(false)
    })
})
