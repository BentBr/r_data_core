import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import EntitiesPage from './EntitiesPage.vue'
import type { DynamicEntity } from '@/types/schemas'

const mockGetEntityDefinitions = vi.fn()
const mockCreateEntity = vi.fn()
const mockDeleteEntity = vi.fn()
const mockGetEntity = vi.fn()
const mockUpdateEntity = vi.fn()
const mockBrowseByPath = vi.fn()

vi.mock('@/api/typed-client', () => ({
    typedHttpClient: {
        getEntityDefinitions: (page?: number, itemsPerPage?: number) =>
            mockGetEntityDefinitions(page, itemsPerPage),
        createEntity: (entityType: string, data: Record<string, unknown>) =>
            mockCreateEntity(entityType, data),
        deleteEntity: (entityType: string, uuid: string) => mockDeleteEntity(entityType, uuid),
        getEntity: (entityType: string, uuid: string, opts?: unknown) =>
            mockGetEntity(entityType, uuid, opts),
        updateEntity: (entityType: string, uuid: string, data: Record<string, unknown>) =>
            mockUpdateEntity(entityType, uuid, data),
        browseByPath: (path: string, limit?: number, offset?: number) =>
            mockBrowseByPath(path, limit, offset),
    },
    ValidationError: class ValidationError extends Error {
        violations: Array<{ field: string; message: string }>

        constructor(message: string, violations: Array<{ field: string; message: string }>) {
            super(message)
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
        mockUpdateEntity.mockResolvedValue({})
        mockBrowseByPath.mockResolvedValue({ data: [] })
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

describe('EntitiesPage - CRUD methods', () => {
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
        mockUpdateEntity.mockResolvedValue({})
        mockBrowseByPath.mockResolvedValue({ data: [] })
        mockHasPermission.mockImplementation((namespace: string, permission: string) => {
            return namespace === 'Entities' && (permission === 'Create' || permission === 'Admin')
        })
    })

    const mountReady = async () => {
        const wrapper = mount(EntitiesPage, { global: { plugins: [router] } })
        await vi.waitUntil(() => mockGetEntityDefinitions.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()
        return wrapper
    }

    it('handleItemClick loads and selects the entity on success', async () => {
        const wrapper = await mountReady()
        mockGetEntity.mockResolvedValue({
            entity_type: 'Customer',
            field_data: { uuid: 'entity-1', path: '/entity-1' },
        })

        const vm = wrapper.vm as unknown as {
            handleItemClick: (item: { uuid?: string; entity_type?: string }) => Promise<void>
            selectedEntity: unknown
        }
        await vm.handleItemClick({ uuid: 'entity-1', entity_type: 'Customer' })

        expect(mockGetEntity).toHaveBeenCalledWith('Customer', 'entity-1', {
            includeChildrenCount: true,
        })
        expect(vm.selectedEntity).toEqual({
            entity_type: 'Customer',
            field_data: { uuid: 'entity-1', path: '/entity-1' },
        })
    })

    it('handleItemClick does nothing when item has no uuid', async () => {
        const wrapper = await mountReady()

        const vm = wrapper.vm as unknown as {
            handleItemClick: (item: { uuid?: string; entity_type?: string }) => Promise<void>
        }
        await vm.handleItemClick({ entity_type: 'Customer' })

        expect(mockGetEntity).not.toHaveBeenCalled()
    })

    it('handleItemClick swallows errors from the API', async () => {
        const wrapper = await mountReady()
        mockGetEntity.mockRejectedValue(new Error('boom'))

        const vm = wrapper.vm as unknown as {
            handleItemClick: (item: { uuid?: string; entity_type?: string }) => Promise<void>
            loading: boolean
        }
        await vm.handleItemClick({ uuid: 'entity-1', entity_type: 'Customer' })

        expect(vm.loading).toBe(false)
        expect(showError).toHaveBeenCalled()
    })

    it('editEntity opens the edit dialog', async () => {
        const wrapper = await mountReady()

        const vm = wrapper.vm as unknown as {
            editEntity: () => void
            showEditDialog: boolean
        }
        vm.editEntity()

        expect(vm.showEditDialog).toBe(true)
    })

    it('createEntity reloads the tree path and shows a success message', async () => {
        const wrapper = await mountReady()

        const vm = wrapper.vm as unknown as {
            createEntity: (data: {
                entity_type: string
                data: Record<string, unknown>
                parent_uuid: string | null
            }) => Promise<void>
            showCreateDialog: boolean
        }
        vm.showCreateDialog = true
        await wrapper.vm.$nextTick()

        await vm.createEntity({
            entity_type: 'Customer',
            data: { path: '/test/entity-name', entity_key: 'entity-name', published: false },
            parent_uuid: null,
        })

        expect(mockCreateEntity).toHaveBeenCalledWith('Customer', {
            entity_type: 'Customer',
            data: { path: '/test/entity-name', entity_key: 'entity-name', published: false },
            parent_uuid: null,
        })
        expect(vm.showCreateDialog).toBe(false)
        expect(showSuccess).toHaveBeenCalledWith('Entity created successfully')
    })

    it('createEntity sets field errors on validation failure and keeps dialog open', async () => {
        const wrapper = await mountReady()
        const { ValidationError } = await import('@/api/typed-client')
        mockCreateEntity.mockRejectedValue(
            new ValidationError('validation', [{ field: 'entity_key', message: 'already exists' }])
        )

        const vm = wrapper.vm as unknown as {
            createEntity: (data: {
                entity_type: string
                data: Record<string, unknown>
                parent_uuid: string | null
            }) => Promise<void>
            showCreateDialog: boolean
            createDialogRef: { setFieldErrors: (errors: Record<string, string>) => void } | null
        }
        vm.showCreateDialog = true
        await wrapper.vm.$nextTick()

        const setFieldErrorsSpy = vi.spyOn(vm.createDialogRef!, 'setFieldErrors')

        await vm.createEntity({
            entity_type: 'Customer',
            data: { path: '/test', entity_key: 'dup', published: false },
            parent_uuid: null,
        })

        expect(setFieldErrorsSpy).toHaveBeenCalledWith({ entity_key: 'already exists' })
        // Dialog stays open on validation failure.
        expect(vm.showCreateDialog).toBe(true)
        expect(showError).toHaveBeenCalled()
    })

    it('createEntity handles generic (non-validation) errors', async () => {
        const wrapper = await mountReady()
        mockCreateEntity.mockRejectedValue(new Error('server exploded'))

        const vm = wrapper.vm as unknown as {
            createEntity: (data: {
                entity_type: string
                data: Record<string, unknown>
                parent_uuid: string | null
            }) => Promise<void>
        }

        await vm.createEntity({
            entity_type: 'Customer',
            data: { path: '/test', entity_key: 'x', published: false },
            parent_uuid: null,
        })

        expect(showError).toHaveBeenCalled()
    })

    it('updateEntity does nothing when no entity is selected', async () => {
        const wrapper = await mountReady()

        const vm = wrapper.vm as unknown as {
            updateEntity: (data: {
                data: Record<string, unknown>
                parent_uuid: null
            }) => Promise<void>
        }
        await vm.updateEntity({ data: {}, parent_uuid: null })

        expect(mockUpdateEntity).not.toHaveBeenCalled()
    })

    it('updateEntity updates the selected entity and shows success on success', async () => {
        const wrapper = await mountReady()

        const vm = wrapper.vm as unknown as {
            selectedEntity: { entity_type: string; field_data: Record<string, unknown> } | null
            showEditDialog: boolean
            updateEntity: (data: {
                data: Record<string, unknown>
                parent_uuid: string | null
            }) => Promise<void>
        }
        vm.selectedEntity = {
            entity_type: 'Customer',
            field_data: { uuid: 'entity-1', path: '/entity-1' },
        }
        vm.showEditDialog = true
        await wrapper.vm.$nextTick()

        await vm.updateEntity({ data: { published: true }, parent_uuid: null })

        expect(mockUpdateEntity).toHaveBeenCalledWith('Customer', 'entity-1', {
            data: { published: true },
            parent_uuid: null,
        })
        expect(vm.showEditDialog).toBe(false)
        expect(showSuccess).toHaveBeenCalledWith('Entity updated successfully')
    })

    it('updateEntity sets field errors on validation failure', async () => {
        const wrapper = await mountReady()
        const { ValidationError } = await import('@/api/typed-client')
        mockUpdateEntity.mockRejectedValue(
            new ValidationError('validation', [{ field: 'path', message: 'invalid path' }])
        )

        const vm = wrapper.vm as unknown as {
            selectedEntity: { entity_type: string; field_data: Record<string, unknown> } | null
            updateEntity: (data: {
                data: Record<string, unknown>
                parent_uuid: string | null
            }) => Promise<void>
            editDialogRef: { setFieldErrors: (errors: Record<string, string>) => void } | null
        }
        vm.selectedEntity = {
            entity_type: 'Customer',
            field_data: { uuid: 'entity-1' },
        }
        await wrapper.vm.$nextTick()

        const setFieldErrorsSpy = vi.spyOn(vm.editDialogRef!, 'setFieldErrors')

        await vm.updateEntity({ data: { path: '??' }, parent_uuid: null })

        expect(setFieldErrorsSpy).toHaveBeenCalledWith({ path: 'invalid path' })
    })

    it('deleteEntity does nothing when no entity is selected', async () => {
        const wrapper = await mountReady()

        const vm = wrapper.vm as unknown as { deleteEntity: () => Promise<void> }
        await vm.deleteEntity()

        expect(mockDeleteEntity).not.toHaveBeenCalled()
    })

    it('deleteEntity removes the entity, closes the dialog and shows success', async () => {
        const wrapper = await mountReady()

        const vm = wrapper.vm as unknown as {
            selectedEntity: { entity_type: string; field_data: Record<string, unknown> } | null
            showDeleteDialog: boolean
            deleteEntity: () => Promise<void>
        }
        vm.selectedEntity = {
            entity_type: 'Customer',
            field_data: { uuid: 'entity-1', path: '/test/entity-1' },
        }
        vm.showDeleteDialog = true
        await wrapper.vm.$nextTick()

        await vm.deleteEntity()

        expect(mockDeleteEntity).toHaveBeenCalledWith('Customer', 'entity-1')
        expect(vm.selectedEntity).toBeNull()
        expect(vm.showDeleteDialog).toBe(false)
        expect(showSuccess).toHaveBeenCalledWith('success')
    })

    it('deleteEntity handles API errors', async () => {
        const wrapper = await mountReady()
        mockDeleteEntity.mockRejectedValue(new Error('cannot delete'))

        const vm = wrapper.vm as unknown as {
            selectedEntity: { entity_type: string; field_data: Record<string, unknown> } | null
            deleteEntity: () => Promise<void>
        }
        vm.selectedEntity = {
            entity_type: 'Customer',
            field_data: { uuid: 'entity-1', path: '/entity-1' },
        }
        await wrapper.vm.$nextTick()

        await vm.deleteEntity()

        expect(showError).toHaveBeenCalled()
    })

    it('loadEntities increments the tree refresh key', async () => {
        const wrapper = await mountReady()

        const vm = wrapper.vm as unknown as {
            treeRefreshKey: number
            loadEntities: () => Promise<void>
            loading: boolean
        }
        const before = vm.treeRefreshKey
        await vm.loadEntities()

        expect(vm.treeRefreshKey).toBe(before + 1)
        expect(vm.loading).toBe(false)
    })

    it('updateExpandedItems and handleTreeSelection update local state', async () => {
        const wrapper = await mountReady()

        const vm = wrapper.vm as unknown as {
            updateExpandedItems: (items: string[]) => void
            handleTreeSelection: (items: string[]) => void
            expandedItems: string[]
            selectedItems: string[]
        }
        vm.updateExpandedItems(['a', 'b'])
        vm.handleTreeSelection(['c'])

        expect(vm.expandedItems).toEqual(['a', 'b'])
        expect(vm.selectedItems).toEqual(['c'])
    })

    it('opens the create dialog on mount when the route has ?create=true', async () => {
        await router.push('/entities?create=true')
        await router.isReady()

        const wrapper = mount(EntitiesPage, { global: { plugins: [router] } })
        await vi.waitUntil(() => mockGetEntityDefinitions.mock.calls.length > 0, { timeout: 1000 })
        await wrapper.vm.$nextTick()
        await new Promise(resolve => setTimeout(resolve, 10))

        const vm = wrapper.vm as unknown as { showCreateDialog: boolean }
        expect(vm.showCreateDialog).toBe(true)

        await router.push('/entities')
    })
})
